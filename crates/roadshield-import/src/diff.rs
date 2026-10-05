//! Reviewable pack differences: rules per field, assets by hash, upstream
//! engine sources, and a side-by-side visual report.

use std::collections::{BTreeMap, BTreeSet};

use roadshield::{DisplayContext, Engine, Manifest, Rendering, RouteDescriptor, ShieldDef};
use serde::Serialize;
use serde_json::Value;

/// One changed value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldChange {
    /// JSON path within the definition.
    pub path: String,
    /// Old value (`null` when added).
    pub old: Value,
    /// New value (`null` when removed).
    pub new: Value,
}

/// Rule changes for one network.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NetworkChange {
    /// Network key.
    pub network: String,
    /// Changed fields.
    pub fields: Vec<FieldChange>,
}

/// Added, removed and changed IDs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct IdChanges {
    /// Present only in the new pack.
    pub added: Vec<String>,
    /// Present only in the old pack.
    pub removed: Vec<String>,
    /// Present in both with different content.
    pub changed: Vec<String>,
}

impl IdChanges {
    fn from_maps(old: &BTreeMap<String, String>, new: &BTreeMap<String, String>) -> Self {
        Self {
            added: new
                .keys()
                .filter(|k| !old.contains_key(*k))
                .cloned()
                .collect(),
            removed: old
                .keys()
                .filter(|k| !new.contains_key(*k))
                .cloned()
                .collect(),
            changed: old
                .iter()
                .filter(|(k, v)| new.get(*k).is_some_and(|n| n != *v))
                .map(|(k, _)| k.clone())
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Everything that differs between two packs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackDiff {
    /// Old and new content hashes.
    pub content_hash: (String, String),
    /// Old and new upstream commits.
    pub upstream_commit: (String, String),
    /// Networks (after `bannerMap` expansion).
    pub networks: IdChanges,
    /// Field-level changes of changed networks.
    pub network_changes: Vec<NetworkChange>,
    /// Global option changes.
    pub options: Vec<FieldChange>,
    /// Blank SVGs by hash.
    pub blanks: IdChanges,
    /// Fonts by hash.
    pub fonts: IdChanges,
    /// Licence files by hash.
    pub licenses: IdChanges,
    /// Upstream engine sources by hash; any change needs porting review.
    pub engine_sources: IdChanges,
}

fn value_diff(path: &str, old: &Value, new: &Value, out: &mut Vec<FieldChange>) {
    match (old, new) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for k in keys {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                value_diff(
                    &p,
                    a.get(k).unwrap_or(&Value::Null),
                    b.get(k).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        _ if old != new => out.push(FieldChange {
            path: path.into(),
            old: old.clone(),
            new: new.clone(),
        }),
        _ => {}
    }
}

fn to_value<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn hashes<'a>(items: impl Iterator<Item = (&'a str, &'a str)>) -> BTreeMap<String, String> {
    items.map(|(k, v)| (k.to_owned(), v.to_owned())).collect()
}

fn defs(engine: &Engine) -> BTreeMap<String, Value> {
    let rules = engine.rules();
    rules
        .networks
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.as_ref().map_or(Value::Null, |d: &ShieldDef| to_value(d)),
            )
        })
        .collect()
}

/// Compares two loaded packs.
#[must_use]
pub fn diff_packs(old: &Engine, new: &Engine) -> PackDiff {
    let (mo, mn): (&Manifest, &Manifest) = (old.manifest(), new.manifest());
    let (dold, dnew) = (defs(old), defs(new));
    let def_hashes = |d: &BTreeMap<String, Value>| -> BTreeMap<String, String> {
        d.iter().map(|(k, v)| (k.clone(), v.to_string())).collect()
    };
    let networks = IdChanges::from_maps(&def_hashes(&dold), &def_hashes(&dnew));
    let network_changes = networks
        .changed
        .iter()
        .map(|n| {
            let mut fields = Vec::new();
            value_diff(
                "",
                dold.get(n).unwrap_or(&Value::Null),
                dnew.get(n).unwrap_or(&Value::Null),
                &mut fields,
            );
            NetworkChange {
                network: n.clone(),
                fields,
            }
        })
        .collect();
    let mut options = Vec::new();
    value_diff(
        "",
        &to_value(&old.rules().options),
        &to_value(&new.rules().options),
        &mut options,
    );
    let blank_h = |m: &Manifest| {
        hashes(
            m.blanks
                .iter()
                .map(|b| (b.id.as_str(), b.file.blake3.as_str())),
        )
    };
    let font_h = |m: &Manifest| {
        hashes(
            m.fonts
                .iter()
                .map(|f| (f.id.as_str(), f.file.blake3.as_str())),
        )
    };
    let lic_h = |m: &Manifest| {
        hashes(
            m.licenses
                .iter()
                .map(|l| (l.file.path.as_str(), l.file.blake3.as_str())),
        )
    };
    let src_h = |m: &Manifest| {
        hashes(
            m.upstream
                .engine_sources
                .iter()
                .map(|s| (s.path.as_str(), s.sha256.as_str())),
        )
    };
    PackDiff {
        content_hash: (mo.content_hash.clone(), mn.content_hash.clone()),
        upstream_commit: (mo.upstream.commit.clone(), mn.upstream.commit.clone()),
        networks,
        network_changes,
        options,
        blanks: IdChanges::from_maps(&blank_h(mo), &blank_h(mn)),
        fonts: IdChanges::from_maps(&font_h(mo), &font_h(mn)),
        licenses: IdChanges::from_maps(&lic_h(mo), &lic_h(mn)),
        engine_sources: IdChanges::from_maps(&src_h(mo), &src_h(mn)),
    }
}

impl PackDiff {
    /// True when the packs are identical in every compared respect.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.networks.is_empty()
            && self.options.is_empty()
            && self.blanks.is_empty()
            && self.fonts.is_empty()
            && self.licenses.is_empty()
            && self.engine_sources.is_empty()
    }

    /// Markdown summary for review.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        push_line(
            &mut s,
            &format!(
                "# Pack diff\n\n- content: `{}` → `{}`",
                self.content_hash.0, self.content_hash.1
            ),
        );
        push_line(
            &mut s,
            &format!(
                "- upstream: `{}` → `{}`",
                self.upstream_commit.0, self.upstream_commit.1
            ),
        );
        let section = |s: &mut String, title: &str, c: &IdChanges| {
            if c.is_empty() {
                return;
            }
            push_line(s, &format!("\n## {title}\n"));
            for (label, ids) in [
                ("added", &c.added),
                ("removed", &c.removed),
                ("changed", &c.changed),
            ] {
                if !ids.is_empty() {
                    push_line(s, &format!("- {label} ({}): {}", ids.len(), ids.join(", ")));
                }
            }
        };
        section(
            &mut s,
            "Upstream engine sources (port review required)",
            &self.engine_sources,
        );
        section(&mut s, "Networks", &self.networks);
        for nc in &self.network_changes {
            push_line(&mut s, &format!("\n### {}\n", nc.network));
            for f in &nc.fields {
                push_line(
                    &mut s,
                    &format!("- `{}`: `{}` → `{}`", f.path, f.old, f.new),
                );
            }
        }
        if !self.options.is_empty() {
            push_line(&mut s, "\n## Options\n");
            for f in &self.options {
                push_line(
                    &mut s,
                    &format!("- `{}`: `{}` → `{}`", f.path, f.old, f.new),
                );
            }
        }
        section(&mut s, "Blanks", &self.blanks);
        section(&mut s, "Fonts", &self.fonts);
        section(&mut s, "Licences", &self.licenses);
        if self.is_empty() {
            s.push_str("\nNo differences.\n");
        }
        s
    }
}

fn push_line(s: &mut String, line: &str) {
    s.push_str(line);
    s.push('\n');
}

const SAMPLE_REFS: &[&str] = &["1", "22", "287", "1234"];

fn cell(engine: &Engine, route: &RouteDescriptor) -> String {
    match engine.render(
        route,
        &DisplayContext {
            scale: 2.0,
            ..DisplayContext::default()
        },
    ) {
        Ok(Rendering::Symbol(sym)) => format!(
            "<img alt=\"\" src=\"data:image/svg+xml;base64,{}\">",
            base64_encode(sym.svg.as_bytes())
        ),
        Ok(Rendering::NoShield { reason, .. }) => {
            format!("<em>no shield: {}</em>", escape(&format!("{reason:?}")))
        }
        Err(e) => format!("<strong>error: {}</strong>", escape(&e.to_string())),
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18u32, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(T[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Networks worth rendering: changed/added rules plus users of changed blanks.
fn affected_networks(new: &Engine, diff: &PackDiff) -> Vec<String> {
    let mut set: BTreeSet<String> = diff
        .networks
        .changed
        .iter()
        .chain(&diff.networks.added)
        .cloned()
        .collect();
    let changed_blanks: BTreeSet<&String> = diff.blanks.changed.iter().collect();
    for (k, v) in &new.rules().networks {
        let uses = v
            .as_ref()
            .and_then(|d| d.sprite_blank.as_ref())
            .is_some_and(|b| {
                b.ids()
                    .iter()
                    .any(|id| changed_blanks.contains(&(*id).to_owned()))
            });
        if uses {
            set.insert(k.clone());
        }
    }
    set.into_iter().collect()
}

/// Self-contained HTML page rendering affected networks with both packs.
#[must_use]
pub fn visual_report_html(old: &Engine, new: &Engine, diff: &PackDiff) -> String {
    let mut rows = String::new();
    for network in affected_networks(new, diff) {
        for r in SAMPLE_REFS {
            let route = RouteDescriptor::new(network.clone(), *r);
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{r}</td><td>{}</td><td>{}</td></tr>",
                escape(&network),
                cell(old, &route),
                cell(new, &route)
            ));
        }
    }
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" \
         content=\"width=device-width\"><title>Pack visual diff</title><style>body{{font:14px sans-serif;\
         background:#f6f1ea;color:#222;margin:16px}}table{{border-collapse:collapse}}td,th{{border:1px solid \
         #ccc;padding:4px 8px;text-align:left}}</style></head><body><h1>Pack visual diff</h1><p>{} → {}</p>\
         <table><tr><th>network</th><th>ref</th><th>old</th><th>new</th></tr>{rows}</table></body></html>",
        escape(&diff.content_hash.0),
        escape(&diff.content_hash.1)
    )
}

#[cfg(test)]
mod tests;
