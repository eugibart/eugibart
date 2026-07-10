//! Data-driven ECU definitions.
//!
//! Everything model-specific — init method, addresses, timing overrides,
//! live-data channels, DTC tables, service routines and their safety
//! preconditions — lives in TOML files under `definitions/`. Adding support
//! for a new bike means writing a new TOML file, not new code.
//!
//! The schema deliberately has no way to express memory read/write services:
//! ECU map access is out of scope for v1 (see docs/SAFETY.md).

pub mod registry;
pub mod schema;

pub use registry::Registry;
pub use schema::EcuDefinition;

#[derive(Debug, thiserror::Error)]
pub enum DefsError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("parse error in {path}: {source}")]
    Parse {
        path: String,
        source: Box<toml::de::Error>,
    },
    #[error("duplicate ECU definition id: {0}")]
    DuplicateId(String),
    #[error("invalid definition {id}: {reason}")]
    Invalid { id: String, reason: String },
}

pub type Result<T> = std::result::Result<T, DefsError>;
