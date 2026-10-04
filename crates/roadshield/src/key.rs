//! Deterministic semantic cache keys.

use serde::Serialize;

use crate::route::{DisplayContext, RouteDescriptor};

/// Bumped whenever the engine's output for identical inputs changes.
pub const ENGINE_OUTPUT_VERSION: &str = "roadshield-1";

#[derive(Serialize)]
struct KeyInput<'a> {
    engine: &'a str,
    pack: &'a str,
    network: Option<&'a str>,
    #[serde(rename = "ref")]
    ref_: Option<&'a str>,
    name: Option<&'a str>,
    ctx: &'a DisplayContext,
}

/// Key over every input that can change the output: engine version, pack
/// content hash, network/ref/name and the display context. `colour`,
/// `extra` and `source` are excluded because no rule reads them, and
/// `expected_pack` because it only gates whether rendering happens, so
/// pinned and unpinned callers share cache entries.
pub fn semantic_key(
    pack_content_hash: &str,
    route: &RouteDescriptor,
    ctx: &DisplayContext,
) -> String {
    let ctx = &DisplayContext {
        expected_pack: None,
        ..ctx.clone()
    };
    let input = KeyInput {
        engine: ENGINE_OUTPUT_VERSION,
        pack: pack_content_hash,
        network: route.network.as_deref(),
        ref_: route.ref_.as_deref(),
        name: route.name.as_deref(),
        ctx,
    };
    // Serialising plain data cannot fail; an empty input still hashes.
    let bytes = serde_json::to_vec(&input).unwrap_or_default();
    let hex = blake3::hash(&bytes).to_hex();
    format!("rs1_{}", hex.get(..32).unwrap_or(hex.as_str()))
}
