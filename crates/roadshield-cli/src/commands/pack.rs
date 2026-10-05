//! `roadshield import|subset|diff|inspect`.

use anyhow::{Context as _, Result};
use clap::ArgMatches;
use roadshield_import::{
    BuildRequest, CheckRequest, CheckStatus, ImportConfig, SubsetRequest, build_pack_blocking,
    cut_subset, diff_packs, load_engine_blocking, load_pack_dir_blocking, read_blocking,
    upstream_check_blocking, visual_report_html, write_blocking,
};
use serde_json::json;

use super::{load_engine, path, print_json, write_out};

/// Builds a pack from pinned inputs.
pub fn import(m: &ArgMatches) -> Result<()> {
    let config_path = path(m, "config")?;
    let config: ImportConfig = serde_json::from_slice(&read_blocking(&config_path)?)
        .with_context(|| format!("parsing {}", config_path.display()))?;
    let (checkout, inputs, out) = (path(m, "checkout")?, path(m, "inputs")?, path(m, "out")?);
    let report = build_pack_blocking(&BuildRequest {
        config: &config,
        checkout: &checkout,
        inputs: &inputs,
        out_dir: &out,
        config_dir: config_path.parent().unwrap_or(std::path::Path::new(".")),
    })?;
    for (cat, names) in &report.inventory.stale {
        tracing::warn!(
            "roadshield implements {cat} names upstream no longer declares: {}",
            names.join(", ")
        );
    }
    tracing::info!(
        "wrote pack {} ({}) to {}",
        report.pack_id,
        report.content_hash,
        out.display()
    );
    print_json(&report)
}

/// Cuts a subset pack.
pub fn subset(m: &ArgMatches) -> Result<()> {
    let parent = load_pack_dir_blocking(&path(m, "pack")?)?;
    let networks: Vec<String> = m
        .get_many::<String>("networks")
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let files = cut_subset(&parent, &SubsetRequest { networks })?;
    let out = path(m, "out")?;
    if out.join(roadshield::MANIFEST_PATH).is_file() {
        std::fs::remove_dir_all(&out).with_context(|| format!("replacing {}", out.display()))?;
    }
    for (rel, bytes) in &files {
        write_blocking(&out.join(rel), bytes)?;
    }
    let engine = load_engine_blocking(&out)?;
    let m = engine.manifest();
    print_json(&json!({
        "pack": m.id,
        "content_hash": m.content_hash,
        "networks": engine.networks().count(),
        "excluded": m.subset.as_ref().map_or(0, |s| s.excluded_networks.len()),
        "blanks": m.blanks.len(),
    }))
}

/// Diffs two packs.
pub fn diff(m: &ArgMatches) -> Result<()> {
    let old = load_engine_blocking(&path(m, "old")?)?;
    let new = load_engine_blocking(&path(m, "new")?)?;
    let d = diff_packs(&old, &new);
    if let Some(p) = m.get_one::<std::path::PathBuf>("html") {
        write_blocking(p, visual_report_html(&old, &new, &d).as_bytes())?;
    }
    if let Some(p) = m.get_one::<std::path::PathBuf>("json") {
        write_blocking(p, &serde_json::to_vec_pretty(&d)?)?;
    }
    write_out(None, d.to_markdown().as_bytes())
}

/// Summarises a pack.
pub fn inspect(m: &ArgMatches) -> Result<()> {
    let engine = load_engine(m)?;
    let man = engine.manifest();
    let issues = engine.validate();
    let mut summary = json!({
        "id": man.id,
        "content_hash": man.content_hash,
        "upstream": man.upstream.commit,
        "rules_source": man.upstream.rules_source.url,
        "networks": engine.networks().count(),
        "blanks": man.blanks.len(),
        "fonts": man.fonts.iter().map(|f| json!({"id": f.id, "license": f.license})).collect::<Vec<_>>(),
        "licenses": man.licenses.iter().map(|l| json!({"id": l.id, "applies_to": l.applies_to, "attribution": l.attribution})).collect::<Vec<_>>(),
        "subset": man.subset.as_ref().map(|s| s.excluded_networks.len()),
        "issues": issues,
    });
    if m.get_flag("networks") {
        summary["network_keys"] = json!(engine.networks().collect::<Vec<_>>());
    }
    print_json(&summary)
}

/// Checks a newer upstream checkout; fails when the engine or the
/// extensions must adapt.
pub fn upstream_check(m: &ArgMatches) -> Result<()> {
    let config_path = path(m, "config")?;
    let config: ImportConfig = serde_json::from_slice(&read_blocking(&config_path)?)
        .with_context(|| format!("parsing {}", config_path.display()))?;
    let pinned = read_blocking(&path(m, "inputs")?.join(&config.rules.file)).ok();
    let (checkout, rules) = (path(m, "checkout")?, path(m, "rules")?);
    let check = upstream_check_blocking(
        &CheckRequest {
            config: &config,
            config_dir: config_path.parent().unwrap_or(std::path::Path::new(".")),
            checkout: &checkout,
            rules: &rules,
        },
        pinned.as_deref(),
    )?;
    let report = check.to_markdown();
    if let Some(p) = m.get_one::<std::path::PathBuf>("report") {
        write_blocking(p, report.as_bytes())?;
    }
    if let Some(p) = m.get_one::<std::path::PathBuf>("json") {
        write_blocking(p, &serde_json::to_vec_pretty(&check)?)?;
    }
    write_out(None, report.as_bytes())?;
    if check.status == CheckStatus::Incompatible {
        anyhow::bail!("upstream is incompatible with the engine or the pack's extensions");
    }
    Ok(())
}
