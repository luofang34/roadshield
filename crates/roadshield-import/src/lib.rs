//! Builds roadshield resource packs from a pinned OSM Americana checkout and
//! pinned input files, cuts verifiable subsets, and diffs packs for review.
//!
//! Every input is checked against a SHA-256 pinned in the import config; the
//! upstream checkout must be at the pinned commit. No upstream code runs.

mod build;
mod config;
mod diff;
mod error;
mod files;
mod inventory;
mod subset;

pub use build::{BuildReport, BuildRequest, build_pack_blocking};
pub use config::{FontInput, FontLicense, ImportConfig, PinnedInput, UpstreamLicense, UpstreamPin};
pub use diff::{PackDiff, diff_packs, visual_report_html};
pub use error::ImportError;
pub use files::{
    load_engine_blocking, load_pack_dir_blocking, read_blocking, sha256_hex, write_blocking,
};
pub use inventory::{Inventory, inventory_blocking, known_def_fields, known_param_fields};
pub use subset::{SubsetRequest, cut_subset};
