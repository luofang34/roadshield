//! The public engine: a loaded pack plus pure render calls.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::blank::PreparedBlank;
use crate::compose::{ComposeEnv, Composed, compose};
use crate::document;
use crate::error::{PackError, ShieldError, Warning};
use crate::font::{FontFace, FontStack};
use crate::key::{ENGINE_OUTPUT_VERSION, semantic_key};
use crate::pack::{Manifest, ResourcePack};
use crate::route::{DisplayContext, InputLimits, RouteDescriptor, UnknownNetworkPolicy};
use crate::select::{SelectError, Selection, select};
use crate::symbol::{Dependency, DependencyKind, Provenance, Rendering};
use crate::validate::{IssueKind, RuleIssue, check_def, validate};

/// Canvas-style text metrics (see [`Engine::measure_text`]).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct TextMetrics {
    /// Advance width.
    pub width: f64,
    /// Distance from the alignment point to the ink's left edge (positive leftward).
    pub actual_bounding_box_left: f64,
    /// Distance from the alignment point to the ink's right edge.
    pub actual_bounding_box_right: f64,
    /// Distance from the top baseline up to the ink's top edge.
    pub actual_bounding_box_ascent: f64,
    /// Distance from the top baseline down to the ink's bottom edge.
    pub actual_bounding_box_descent: f64,
}

/// Shield generator over one resource pack. Immutable and `Send + Sync`;
/// every call is a pure function of its inputs.
#[derive(Debug, Clone)]
pub struct Engine {
    manifest: Manifest,
    rules: crate::model::ShieldSpec,
    blanks: HashMap<String, PreparedBlank>,
    blank_hashes: HashMap<String, String>,
    faces: IndexMap<String, FontFace>,
    font_hashes: HashMap<String, String>,
    excluded: HashSet<String>,
    limits: InputLimits,
}

impl Engine {
    /// Prepares fonts and blanks from a verified pack.
    ///
    /// # Errors
    ///
    /// Fails with [`PackError`] when a listed blank or font is missing, a blank
    /// is not a valid SVG within limits, or a font cannot be parsed.
    pub fn new(pack: ResourcePack) -> Result<Self, PackError> {
        let mut blanks = HashMap::new();
        let mut blank_hashes = HashMap::new();
        for entry in &pack.manifest.blanks {
            let bytes = pack
                .blanks
                .get(&entry.id)
                .ok_or_else(|| PackError::MissingFile {
                    path: entry.file.path.clone(),
                })?;
            let prepared = PreparedBlank::prepare(&entry.id, bytes, entry.width, entry.height)?;
            blanks.insert(entry.id.clone(), prepared);
            blank_hashes.insert(entry.id.clone(), entry.file.blake3.clone());
        }
        let mut faces = IndexMap::new();
        let mut font_hashes = HashMap::new();
        for entry in &pack.manifest.fonts {
            let bytes = pack
                .fonts
                .get(&entry.id)
                .ok_or_else(|| PackError::MissingFile {
                    path: entry.file.path.clone(),
                })?;
            faces.insert(entry.id.clone(), FontFace::parse(&entry.id, bytes.clone())?);
            font_hashes.insert(entry.id.clone(), entry.file.blake3.clone());
        }
        let excluded = pack
            .manifest
            .subset
            .as_ref()
            .map(|s| s.excluded_networks.iter().cloned().collect())
            .unwrap_or_default();
        Ok(Self {
            manifest: pack.manifest,
            rules: pack.rules,
            blanks,
            blank_hashes,
            faces,
            font_hashes,
            excluded,
            limits: InputLimits::default(),
        })
    }

    /// Replaces the input limits.
    #[must_use]
    pub fn with_limits(mut self, limits: InputLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The pack manifest.
    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Rules with `bannerMap` expanded.
    #[must_use]
    pub fn rules(&self) -> &crate::model::ShieldSpec {
        &self.rules
    }

    /// Network keys with rules, after `bannerMap` expansion.
    pub fn networks(&self) -> impl Iterator<Item = &str> {
        self.rules.networks.keys().map(String::as_str)
    }

    /// Every rule problem in the pack.
    #[must_use]
    pub fn validate(&self) -> Vec<RuleIssue> {
        validate(&self.rules, &self.blanks)
    }

    /// Semantic cache key for a request, without rendering.
    #[must_use]
    pub fn semantic_key(&self, route: &RouteDescriptor, ctx: &DisplayContext) -> String {
        // An explicit copy of the default stack renders identically to none.
        if ctx.font_stack.as_ref() == Some(&self.manifest.font_stack) {
            let normalized = DisplayContext {
                font_stack: None,
                ..ctx.clone()
            };
            return semantic_key(&self.manifest.content_hash, route, &normalized);
        }
        semantic_key(&self.manifest.content_hash, route, ctx)
    }

    fn stack(&self, ctx: &DisplayContext) -> Result<FontStack, ShieldError> {
        let ids = ctx.font_stack.as_ref().unwrap_or(&self.manifest.font_stack);
        let mut faces = Vec::with_capacity(ids.len());
        for id in ids {
            let face = self
                .faces
                .get(id)
                .ok_or_else(|| ShieldError::InvalidInput {
                    field: "font_stack".into(),
                    reason: format!("font {id:?} is not in pack {:?}", self.manifest.id),
                })?;
            faces.push(face.clone());
        }
        FontStack::new(faces).ok_or_else(|| ShieldError::ResourceNotReady {
            detail: "empty font stack".into(),
        })
    }

    fn check_context(&self, ctx: &DisplayContext) -> Result<(), ShieldError> {
        if let Some(exp) = &ctx.expected_pack {
            let hash_ok = exp
                .content_hash
                .as_ref()
                .is_none_or(|h| *h == self.manifest.content_hash);
            if exp.id != self.manifest.id || !hash_ok {
                return Err(ShieldError::ResourceNotReady {
                    detail: format!(
                        "expected pack {:?} ({:?}), loaded {:?} ({})",
                        exp.id, exp.content_hash, self.manifest.id, self.manifest.content_hash
                    ),
                });
            }
        }
        if let Some(theme) = &ctx.theme
            && !self.manifest.themes.contains(theme)
        {
            return Err(ShieldError::InvalidInput {
                field: "theme".into(),
                reason: format!("theme {theme:?} is not in pack {:?}", self.manifest.id),
            });
        }
        Ok(())
    }

    /// Canvas `measureText` quantities for `text` at `font_px` with
    /// `textAlign = "left"` and `textBaseline = "top"`, as the engine models
    /// them. Exposed for conformance checks against a browser.
    ///
    /// # Errors
    ///
    /// Fails with [`ShieldError::InvalidInput`] for an unknown font ID in the
    /// context and [`ShieldError::MissingGlyph`] when no font covers a character
    /// under the context's missing-glyph policy.
    pub fn measure_text(
        &self,
        text: &str,
        font_px: f64,
        ctx: &DisplayContext,
    ) -> Result<TextMetrics, ShieldError> {
        let stack = self.stack(ctx)?;
        let shaped = stack.shape(
            text,
            ctx.direction,
            ctx.language.as_deref(),
            ctx.missing_glyph,
        )?;
        let top = crate::font::top_px(stack.primary().map_or(0.8, FontFace::top_em), font_px);
        let ink = shaped.ink.unwrap_or(crate::font::InkBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        });
        Ok(TextMetrics {
            width: shaped.advance * font_px,
            actual_bounding_box_left: -ink.min_x * font_px,
            actual_bounding_box_right: ink.max_x * font_px,
            actual_bounding_box_ascent: ink.max_y * font_px - top,
            actual_bounding_box_descent: top - ink.min_y * font_px,
        })
    }

    /// Rule selection plus definition checks; `Ok(Err(_))` is a no-shield
    /// outcome.
    fn select_rule(
        &self,
        route: &RouteDescriptor,
        ctx: &DisplayContext,
        network: &str,
        key: &str,
    ) -> Result<Result<Selection, Rendering>, ShieldError> {
        if self.excluded.contains(network) {
            return Err(ShieldError::NetworkNotInPack {
                network: network.into(),
                pack: self.manifest.id.clone(),
            });
        }
        let allow_fallback = ctx.unknown_network == UnknownNetworkPolicy::GenericFallback;
        let sel = match select(&self.rules.networks, route, allow_fallback) {
            Ok(sel) => sel,
            Err(SelectError::NoShield(reason)) => {
                return Ok(Err(Rendering::NoShield {
                    reason,
                    semantic_key: key.into(),
                }));
            }
            Err(SelectError::Unknown(network)) => {
                return Err(ShieldError::UnknownNetwork { network });
            }
        };
        let exists = |id: &str| self.blanks.contains_key(id);
        // Missing blanks are reported for the blank actually chosen.
        let issue = check_def(&sel.def, &exists)
            .into_iter()
            .find(|(k, _)| *k != IssueKind::MissingBlank);
        if let Some((_, detail)) = issue {
            return Err(ShieldError::UnsupportedRule {
                network: network.into(),
                detail,
            });
        }
        Ok(Ok(sel))
    }

    fn dependencies(&self, c: &Composed, font_stack: &[String]) -> Vec<Dependency> {
        let mut deps = vec![Dependency {
            kind: DependencyKind::Rules,
            id: self.manifest.rules.path.clone(),
            blake3: self.manifest.rules.blake3.clone(),
        }];
        if let Some(b) = &c.blank {
            deps.push(Dependency {
                kind: DependencyKind::Blank,
                id: b.clone(),
                blake3: self.blank_hashes.get(b).cloned().unwrap_or_default(),
            });
        }
        for id in c.faces_used.iter().filter_map(|fi| font_stack.get(*fi)) {
            deps.push(Dependency {
                kind: DependencyKind::Font,
                id: id.clone(),
                blake3: self.font_hashes.get(id).cloned().unwrap_or_default(),
            });
        }
        deps
    }

    fn provenance(&self, font_stack: Vec<String>) -> Provenance {
        Provenance {
            engine: ENGINE_OUTPUT_VERSION.into(),
            pack_id: self.manifest.id.clone(),
            pack_content_hash: self.manifest.content_hash.clone(),
            upstream_commit: self.manifest.upstream.commit.clone(),
            font_stack,
        }
    }

    /// Renders the shield for `route`.
    ///
    /// # Errors
    ///
    /// Returns a [`ShieldError`] for out-of-bounds input, a pack or theme the
    /// context does not accept, a network excluded from a subset pack, an unknown
    /// network under [`UnknownNetworkPolicy::Unsupported`], a missing blank or
    /// glyph, an unsupported rule, or text that cannot be fitted. Rules that
    /// decline to draw are not errors: they return [`Rendering::NoShield`].
    pub fn render(
        &self,
        route: &RouteDescriptor,
        ctx: &DisplayContext,
    ) -> Result<Rendering, ShieldError> {
        self.limits.check(route, ctx)?;
        self.check_context(ctx)?;
        let key = self.semantic_key(route, ctx);
        let network = route.network.clone().unwrap_or_default();
        let sel = match self.select_rule(route, ctx, &network, &key)? {
            Ok(sel) => sel,
            Err(no_shield) => return Ok(no_shield),
        };
        let stack = self.stack(ctx)?;
        let env = ComposeEnv {
            options: &self.rules.options,
            stack: &stack,
            blanks: &self.blanks,
            ctx,
            network: &network,
            r: f64::from(ctx.pixel_grid),
        };
        let c = compose(&env, &sel)?;
        let font_stack = stack.ids();
        let dependencies = self.dependencies(&c, &font_stack);
        let svg = document::svg(&c, &sel.rule_key, &network, ctx);
        let mut warnings = c.warnings.clone();
        if sel.fallback {
            warnings.insert(0, Warning::GenericFallback { network });
        }
        let symbol = document::symbol(
            c,
            sel,
            ctx.scale / f64::from(ctx.pixel_grid),
            document::Extras {
                svg,
                provenance: self.provenance(font_stack),
                dependencies,
                semantic_key: key,
                warnings,
            },
        );
        Ok(Rendering::Symbol(Box::new(symbol)))
    }
}

#[cfg(test)]
mod tests;
