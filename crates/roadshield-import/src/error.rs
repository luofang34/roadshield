//! Import errors.

use std::path::PathBuf;

/// Failure while building, cutting or diffing a pack.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// File system access failed.
    #[error("{op} {path}: {source}")]
    Io {
        /// Operation attempted.
        op: &'static str,
        /// Path involved.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// An input does not match its pinned SHA-256.
    #[error("{path} sha256 mismatch: pinned {expected}, actual {actual}")]
    Checksum {
        /// Input path.
        path: PathBuf,
        /// Pinned digest.
        expected: String,
        /// Computed digest.
        actual: String,
    },
    /// The upstream checkout is not at the pinned commit.
    #[error("upstream checkout {path} is at {actual}, config pins {expected}")]
    Commit {
        /// Checkout path.
        path: PathBuf,
        /// Pinned commit.
        expected: String,
        /// Checked-out commit.
        actual: String,
    },
    /// JSON parse or schema failure (includes unknown ShieldJSON fields).
    #[error("{path} is not valid: {detail}")]
    Json {
        /// File path.
        path: PathBuf,
        /// Parser message.
        detail: String,
    },
    /// Fonts could not be decoded.
    #[error("font {path}: {detail}")]
    Font {
        /// File path.
        path: PathBuf,
        /// Failure detail.
        detail: String,
    },
    /// Rules reference blanks the upstream checkout lacks.
    #[error("{} blank(s) referenced by rules are missing upstream: {}", missing.len(), missing.join(", "))]
    MissingBlanks {
        /// Missing blank IDs.
        missing: Vec<String>,
    },
    /// A blank's size could not be determined or disagrees with the sprite sheet.
    #[error("blank {id}: {detail}")]
    BlankSize {
        /// Blank ID.
        id: String,
        /// Failure detail.
        detail: String,
    },
    /// Upstream uses semantics the engine does not implement.
    #[error("{} incompatibility(ies) with upstream:\n{}", issues.len(), issues.join("\n"))]
    Incompatible {
        /// One line per issue.
        issues: Vec<String>,
    },
    /// The built pack failed to load in the engine.
    #[error("built pack does not load: {0}")]
    Pack(#[from] roadshield::PackError),
    /// Subset request is invalid.
    #[error("subset: {0}")]
    Subset(String),
}
