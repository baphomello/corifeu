//! Where a value came from.
//!
//! Four levels, highest precedence first (ADR-0029). Tracking the origin of
//! every value is not decoration: without it, four-level precedence is a
//! debugging problem of our own making.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A `--flag` on this invocation.
    Flag(String),
    /// A `CORIFEU_*` environment variable.
    Env(String),
    /// The project's `.corifeu/config.yaml`.
    Project(PathBuf),
    /// The user's global config.
    Global(PathBuf),
    /// Nothing set it; the model's default applies.
    Default,
}

impl Source {
    /// Lower binds tighter. Used only for ordering layers, never exposed.
    pub fn precedence(&self) -> u8 {
        match self {
            Self::Flag(_) => 0,
            Self::Env(_) => 1,
            Self::Project(_) => 2,
            Self::Global(_) => 3,
            Self::Default => 4,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Flag(f) => format!("flag {f}"),
            Self::Env(v) => format!("env {v}"),
            Self::Project(p) => format!("project {}", p.display()),
            Self::Global(p) => format!("global {}", p.display()),
            Self::Default => "default".to_string(),
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// One layer of configuration, already parsed to a generic tree.
#[derive(Debug, Clone)]
pub struct Layer {
    pub source: Source,
    pub value: serde_json::Value,
}

impl Layer {
    pub fn new(source: Source, value: serde_json::Value) -> Self {
        Self { source, value }
    }
}
