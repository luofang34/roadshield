//! Checks a newer upstream checkout against the engine and the pack's
//! extensions without building a pack: new `ShieldJSON` fields, upstream
//! semantics the engine does not implement, missing blanks, and extension
//! networks upstream now defines. Run on upstream's latest commit by a
//! scheduled job so changes become reviewable work instead of surprises.

use std::collections::BTreeSet;
use std::path::Path;

use roadshield::{ExtensionSpec, RuleIssue, ShieldSpec, validate_rules};
use serde::Serialize;

use crate::config::ImportConfig;
use crate::error::ImportError;
use crate::extension::{ExtensionConflict, extension_conflicts, load_extension_blocking};
use crate::files::{read_blocking, sha256_hex};
use crate::inventory::{Inventory, inventory_blocking};

/// Overall outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Upstream rules are byte-identical to the pinned input.
    Current,
    /// Upstream changed and the engine and extensions can take it: bump
    /// the pin and review the pack diff.
    UpdateAvailable,
    /// Upstream changed in a way the engine or the extensions must adapt to.
    Incompatible,
}

/// Inputs to [`upstream_check_blocking`].
pub struct CheckRequest<'a> {
    /// Import configuration (pins and extensions).
    pub config: &'a ImportConfig,
    /// Directory of the import config.
    pub config_dir: &'a Path,
    /// Upstream checkout to check (usually its latest commit).
    pub checkout: &'a Path,
    /// `ShieldJSON` generated from that checkout.
    pub rules: &'a Path,
}

/// What the check found.
#[derive(Debug, Clone, Serialize)]
pub struct UpstreamCheck {
    /// Overall outcome.
    pub status: CheckStatus,
    /// Commit the import config pins.
    pub pinned_commit: String,
    /// SHA-256 of the checked `ShieldJSON`.
    pub rules_sha256: String,
    /// The checked rules failed to parse (e.g. a new field).
    pub schema_error: Option<String>,
    /// Upstream source declarations versus the engine.
    pub inventory: Inventory,
    /// Definitions the engine cannot draw (unsupported values, missing blanks).
    pub rule_issues: Vec<RuleIssue>,
    /// Extension networks upstream now defines.
    pub extension_conflicts: Vec<ExtensionConflict>,
    /// Networks upstream added since the pin.
    pub added_networks: Vec<String>,
    /// Networks upstream removed since the pin.
    pub removed_networks: Vec<String>,
}

fn parse_pinned(inputs_rules: Option<&[u8]>) -> BTreeSet<String> {
    inputs_rules
        .and_then(|b| serde_json::from_slice::<ShieldSpec>(b).ok())
        .map(|s| s.networks.into_keys().collect())
        .unwrap_or_default()
}

/// Runs the check.
///
/// # Errors
///
/// Fails with [`ImportError::Io`] if the checkout's sources or the rules
/// cannot be read and with [`ImportError::Json`] for invalid extension
/// rules. Problems with upstream itself are reported, not returned.
pub fn upstream_check_blocking(
    req: &CheckRequest<'_>,
    pinned_rules: Option<&[u8]>,
) -> Result<UpstreamCheck, ImportError> {
    let bytes = read_blocking(req.rules)?;
    let rules_sha256 = sha256_hex(&bytes);
    let inventory = inventory_blocking(req.checkout)?;
    let mut check = UpstreamCheck {
        status: CheckStatus::Current,
        pinned_commit: req.config.upstream.commit.clone(),
        rules_sha256,
        schema_error: None,
        inventory,
        rule_issues: Vec::new(),
        extension_conflicts: Vec::new(),
        added_networks: Vec::new(),
        removed_networks: Vec::new(),
    };
    match serde_json::from_slice::<ShieldSpec>(&bytes) {
        Err(e) => check.schema_error = Some(e.to_string()),
        Ok(spec) => {
            let icons = req.checkout.join("icons");
            let exists = |id: &str| icons.join(format!("{id}.svg")).is_file();
            check.rule_issues = validate_rules(&spec, &exists);
            if let Some(ext) = load_extension_blocking(req.config, req.config_dir)? {
                let ext: ExtensionSpec = ext.spec;
                check.extension_conflicts = extension_conflicts(&spec, &ext);
            }
            let pinned = parse_pinned(pinned_rules);
            let now: BTreeSet<String> = spec.networks.keys().cloned().collect();
            check.added_networks = now.difference(&pinned).cloned().collect();
            check.removed_networks = pinned.difference(&now).cloned().collect();
        }
    }
    check.status = if check.schema_error.is_some()
        || !check.inventory.missing.is_empty()
        || !check.rule_issues.is_empty()
        || !check.extension_conflicts.is_empty()
    {
        CheckStatus::Incompatible
    } else if check.rules_sha256 == req.config.rules.sha256 {
        CheckStatus::Current
    } else {
        CheckStatus::UpdateAvailable
    };
    Ok(check)
}

impl UpstreamCheck {
    /// Markdown report for an issue or a job summary.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut s = format!(
            "# Upstream check: {:?}\n\n- pinned commit: `{}`\n- checked rules sha256: `{}`\n",
            self.status, self.pinned_commit, self.rules_sha256
        );
        let list = |s: &mut String, title: &str, items: Vec<String>| {
            if !items.is_empty() {
                s.push_str(&format!("\n## {title}\n\n"));
                for i in items {
                    s.push_str(&format!("- {i}\n"));
                }
            }
        };
        if let Some(e) = &self.schema_error {
            list(
                &mut s,
                "ShieldJSON no longer parses (new or changed fields)",
                vec![e.clone()],
            );
        }
        list(
            &mut s,
            "Upstream semantics the engine does not implement",
            self.inventory.issues(),
        );
        list(
            &mut s,
            "Definitions the engine cannot draw",
            self.rule_issues
                .iter()
                .map(|i| format!("`{}` {}: {:?} {}", i.network, i.path, i.kind, i.detail))
                .collect(),
        );
        list(
            &mut s,
            "Extension networks upstream now defines (remove or reconcile the extension)",
            self.extension_conflicts
                .iter()
                .map(|c| {
                    format!(
                        "`{}`: upstream `{}`, extension `{}`",
                        c.network, c.upstream, c.extension
                    )
                })
                .collect(),
        );
        list(
            &mut s,
            "Networks added upstream",
            self.added_networks
                .iter()
                .map(|n| format!("`{n}`"))
                .collect(),
        );
        list(
            &mut s,
            "Networks removed upstream",
            self.removed_networks
                .iter()
                .map(|n| format!("`{n}`"))
                .collect(),
        );
        s
    }
}
