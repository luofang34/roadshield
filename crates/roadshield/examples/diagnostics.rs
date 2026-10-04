//! Every kind of outcome, on real inputs: a symbol, a shield the rules
//! decline to draw, the generic fallback for an unknown network, and typed
//! errors. Nothing is ever reported as a blank image.
//!
//! `cargo run -p roadshield --example diagnostics`

#[path = "support/pack.rs"]
mod pack;

use std::io::Write as _;

use roadshield::{DisplayContext, Rendering, RouteDescriptor, UnknownNetworkPolicy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = pack::engine()?;
    let lenient = DisplayContext::default();
    let strict = DisplayContext {
        unknown_network: UnknownNetworkPolicy::Unsupported,
        ..DisplayContext::default()
    };
    let cases = [
        (
            "Interstate 287",
            RouteDescriptor::new("US:I", "287"),
            &lenient,
        ),
        (
            "ref too long",
            RouteDescriptor::new("US:I", "12345678"),
            &lenient,
        ),
        (
            "unknown network, generic rule",
            RouteDescriptor::new("XX:somewhere", "12"),
            &lenient,
        ),
        (
            "unknown network, strict",
            RouteDescriptor::new("XX:somewhere", "12"),
            &strict,
        ),
        (
            "no font for the text",
            RouteDescriptor::new("US:NJ", "国1"),
            &lenient,
        ),
        (
            "Armenian ref",
            RouteDescriptor::new("US:NJ", "Մ4"),
            &lenient,
        ),
        (
            "name mapped to a ref",
            RouteDescriptor::new("US:KY:Parkway", "").with_name("Audubon Parkway"),
            &lenient,
        ),
    ];
    let mut out = std::io::stdout().lock();
    for (label, route, ctx) in cases {
        let line = match engine.render(&route, ctx) {
            Ok(Rendering::Symbol(s)) => format!(
                "symbol {}x{} rule {} text {:?} warnings {:?}",
                s.width,
                s.height,
                s.rule.rule_key,
                s.text.as_ref().map(|t| t.text.as_str()),
                s.warnings
            ),
            Ok(Rendering::NoShield { reason, .. }) => format!("no shield: {reason:?}"),
            Err(e) => format!("error: {e}"),
        };
        writeln!(out, "{label:32} {line}")?;
    }
    Ok(())
}
