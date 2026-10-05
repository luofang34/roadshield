//! Rule selection: which definition applies to a route and what text it shows.
//!
//! Mirrors `getShieldDef`, `refForDefs`, `getRasterShieldBlank` and
//! `romanizeRef` from Americana's `shield.ts`, including their ordering.

use indexmap::IndexMap;
use serde::Serialize;

use crate::error::NoShieldReason;
use crate::model::{ShieldDef, SpriteBlank};
use crate::route::RouteDescriptor;

/// Longest valid ref, in UTF-16 code units (JS `String.length`).
pub const MAX_REF_UTF16: usize = 7;

/// Key of the generic fallback rule.
pub const DEFAULT_RULE: &str = "default";

/// Override applied while resolving a definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "key", rename_all = "snake_case")]
pub enum AppliedOverride {
    /// `overrideByRef[key]`.
    ByRef(String),
    /// `overrideByName[key]`.
    ByName(String),
    /// `noref` replacement.
    NoRef,
}

/// Result of rule selection.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    /// Network key whose rule was used (`default` for the fallback).
    pub rule_key: String,
    /// True when the generic fallback was used for an unknown network.
    pub fallback: bool,
    /// Fully resolved definition.
    pub def: ShieldDef,
    /// Overrides applied, in order.
    pub overrides: Vec<AppliedOverride>,
    /// Ref after `refForDefs`, used for blank width selection.
    pub resolved_ref: String,
    /// Text to draw, after numbering-system conversion; `None` for notext.
    pub text: Option<String>,
}

/// UTF-16 length, matching JS `String.length`.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `isValidRef`: non-empty and at most [`MAX_REF_UTF16`] code units.
#[must_use]
pub fn is_valid_ref(r: &str) -> bool {
    let n = utf16_len(r);
    n != 0 && n <= MAX_REF_UTF16
}

fn truthy(s: Option<&String>) -> bool {
    s.is_some_and(|s| !s.is_empty())
}

/// `refForDefs`: hard-coded ref, then `refsByName[name]`, then the route ref.
pub fn ref_for_def(route: &RouteDescriptor, def: &ShieldDef) -> String {
    if let Some(r) = def.ref_.as_ref().filter(|r| !r.is_empty()) {
        return r.clone();
    }
    if let (Some(map), Some(name)) = (
        &def.refs_by_name,
        route.name.as_ref().filter(|n| !n.is_empty()),
    ) && let Some(r) = map.get(name).filter(|r| !r.is_empty())
    {
        return r.clone();
    }
    route.ref_.clone().unwrap_or_default()
}

/// Selects and resolves the rule for `route`. `Ok(None)`-like outcomes are
/// returned as [`NoShieldReason`]; the caller handles unknown-network policy
/// through `allow_fallback`.
pub fn select(
    networks: &IndexMap<String, Option<ShieldDef>>,
    route: &RouteDescriptor,
    allow_fallback: bool,
) -> Result<Selection, SelectError> {
    let network = route.network.clone().unwrap_or_default();
    let route_ref = route.ref_.clone().unwrap_or_default();
    let Some(base) = networks.get(&network).and_then(Option::as_ref) else {
        if !allow_fallback {
            return Err(SelectError::Unknown(network));
        }
        if !is_valid_ref(&route_ref) {
            return Err(SelectError::NoShield(
                NoShieldReason::UnknownNetworkWithoutRef { network },
            ));
        }
        let Some(def) = networks.get(DEFAULT_RULE).and_then(Option::as_ref) else {
            return Err(SelectError::Unknown(network));
        };
        return Ok(finish(
            DEFAULT_RULE.into(),
            true,
            def.clone(),
            Vec::new(),
            route,
        ));
    };

    let mut def = base.clone();
    let mut overrides = Vec::new();
    let r = ref_for_def(route, &def);

    if let Some(over) = def.override_by_ref.as_ref().and_then(|m| m.get(&r)) {
        overrides.push(AppliedOverride::ByRef(r.clone()));
        def = def.overlay(&over.clone());
    }
    let name = route.name.clone().unwrap_or_default();
    if let Some(over) = def.override_by_name.as_ref().and_then(|m| m.get(&name)) {
        overrides.push(AppliedOverride::ByName(name.clone()));
        def = def.overlay(&over.clone());
    }
    if !is_valid_ref(&r)
        && let Some(noref) = def.noref.clone()
    {
        overrides.push(AppliedOverride::NoRef);
        def = *noref;
        def.notext = Some(true);
    }
    let has_blank = def.sprite_blank.is_some() || def.shape_blank.is_some();
    let notext = def.notext.unwrap_or(false);
    if !is_valid_ref(&r)
        && !truthy(def.ref_.as_ref())
        && !(def.refs_by_name.is_some() && truthy(route.name.as_ref()))
        && (!notext || !has_blank)
    {
        return Err(SelectError::NoShield(NoShieldReason::InvalidRef {
            network,
            utf16_len: utf16_len(&r),
        }));
    }
    Ok(finish(network, false, def, overrides, route))
}

fn finish(
    rule_key: String,
    fallback: bool,
    def: ShieldDef,
    overrides: Vec<AppliedOverride>,
    route: &RouteDescriptor,
) -> Selection {
    let resolved_ref = ref_for_def(route, &def);
    let text = if def.notext.unwrap_or(false) {
        None
    } else if def.numbering_system.as_deref() == Some("roman") && !resolved_ref.is_empty() {
        Some(romanize(&resolved_ref))
    } else {
        Some(resolved_ref.clone())
    };
    Selection {
        rule_key,
        fallback,
        def,
        overrides,
        resolved_ref,
        text,
    }
}

/// Selection failure that is not a rendering error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectError {
    /// No rule and fallback not allowed (or no `default` rule).
    Unknown(String),
    /// Rules say draw nothing.
    NoShield(NoShieldReason),
}

/// JS `parseInt(s, 10)` on the leading integer, `None` for NaN.
fn parse_int_prefix(s: &str) -> Option<(i64, usize)> {
    let t = s.trim_start();
    let (neg, digits) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    let num = digits.get(..end).filter(|d| !d.is_empty())?;
    let mut v: i64 = 0;
    for b in num.bytes() {
        v = v.saturating_mul(10).saturating_add(i64::from(b - b'0'));
    }
    let v = if neg { -v } else { v };
    Some((v, v.to_string().len()))
}

/// `romanizeRef`: leading integer as Roman numerals, keeping the remainder
/// after `String(number).length` code units (an upstream quirk: `"007"`
/// keeps `"07"`). Non-numeric and non-positive refs are returned unchanged
/// (upstream throws for most negatives).
pub fn romanize(r: &str) -> String {
    let Some((n, num_len)) = parse_int_prefix(r) else {
        return r.to_owned();
    };
    if n < 0 {
        return r.to_owned();
    }
    let rep = |s: &str, k: i64| s.repeat(usize::try_from(k).unwrap_or(0));
    let mut roman = rep("M", n / 1000)
        + &rep("D", (n % 1000) / 500)
        + &rep("C", (n % 500) / 100)
        + &rep("L", (n % 100) / 50)
        + &rep("X", (n % 50) / 10)
        + &rep("V", (n % 10) / 5)
        + &rep("I", n % 5);
    for (from, to) in [
        ("DCCCC", "CM"),
        ("CCCC", "CD"),
        ("LXXXX", "XC"),
        ("XXXX", "XL"),
        ("VIIII", "IX"),
        ("IIII", "IV"),
    ] {
        roman = roman.replacen(from, to, 1);
    }
    let units: Vec<u16> = r.encode_utf16().collect();
    let rest = units
        .get(num_len..)
        .map(String::from_utf16_lossy)
        .unwrap_or_default();
    roman + &rest
}

fn is_narrow(c: char) -> bool {
    matches!(c, '1' | 'I' | 'J' | 'i' | 'j' | 'l' | ' ' | '.' | '-')
}

/// `getRasterShieldBlank`: picks the blank whose `_N` suffix equals the
/// weighted character count (narrow characters count ⅔).
pub fn choose_blank<'a>(blank: &'a SpriteBlank, r: &str) -> Option<&'a str> {
    match blank {
        SpriteBlank::One(id) => Some(id),
        SpriteBlank::Many(ids) => {
            let narrow = r.chars().filter(|&c| is_narrow(c)).count();
            #[allow(clippy::cast_precision_loss)]
            let weighted = (utf16_len(r) as f64 - narrow as f64 / 3.0).ceil();
            let optimal: Vec<Option<f64>> = ids
                .iter()
                .map(|id| {
                    let suffix = id.rsplit('_').next().unwrap_or("");
                    #[allow(clippy::cast_precision_loss)]
                    parse_int_prefix(suffix).map(|(v, _)| v as f64)
                })
                .collect();
            let last = ids.len().checked_sub(1)?;
            let idx = if optimal
                .last()
                .copied()
                .flatten()
                .is_some_and(|o| weighted > o)
            {
                last
            } else {
                optimal
                    .iter()
                    .position(|o| *o == Some(weighted))
                    .unwrap_or(0)
            };
            ids.get(idx).map(String::as_str)
        }
    }
}

#[cfg(test)]
mod tests;
