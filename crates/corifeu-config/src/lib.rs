//! Corifeu's configuration: model, layered loading, and validation.
//!
//! Loading is three steps, and the split is what makes errors useful:
//!
//! 1. **Parse** each layer of YAML to a generic tree. Syntax errors are
//!    reported by `serde-saphyr` with a line, a column and a source excerpt
//!    (ADR-0040), attributed to the file they came from.
//! 2. **Merge** the layers, recording which one set each leaf (ADR-0029).
//! 3. **Deserialize** the merged tree into the typed model. A failure here
//!    carries the field path, so the error can name the layer that contributed
//!    the offending value rather than just the field.

pub mod env;
pub mod explain;
pub mod merge;
pub mod model;
pub mod source;
pub mod validate;

use serde::Deserialize;
use std::path::{Path, PathBuf};

pub use model::Config;
pub use source::{Layer, Source};
pub use validate::{Problem, Severity};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A YAML syntax problem. The message carries saphyr's rendering, which
    /// includes the line, the column and the offending source line.
    #[error("{source_label}:\n{message}")]
    Syntax {
        source_label: String,
        message: String,
    },

    /// The YAML parsed but does not fit the model. `origin` names the layer
    /// that contributed the value, which is the part a user cannot work out
    /// from the message alone.
    #[error("{path}: {message} (set by {origin})")]
    Shape {
        path: String,
        message: String,
        origin: String,
    },

    #[error("configuration is not valid:\n{}", .0.iter().map(|p| format!("  {p}")).collect::<Vec<_>>().join("\n"))]
    Invalid(Vec<Problem>),
}

/// A loaded configuration, with everything `--explain` needs.
#[derive(Debug)]
pub struct Loaded {
    pub config: Config,
    pub merged: merge::Merged,
    /// Warnings. Errors are returned as [`Error::Invalid`] instead.
    pub warnings: Vec<Problem>,
}

impl Loaded {
    /// Effective values with their sources, defaults included.
    pub fn entries(&self) -> Result<Vec<explain::Entry>, Error> {
        let effective = serde_json::to_value(&self.config).map_err(|e| Error::Shape {
            path: "<root>".into(),
            message: e.to_string(),
            origin: "internal".into(),
        })?;
        Ok(explain::entries(&effective, &self.merged))
    }
}

/// Parse one YAML file into a layer.
pub fn layer_from_file(path: &Path, source: Source) -> Result<Layer, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::Read {
        path: path.to_path_buf(),
        source: e,
    })?;
    layer_from_str(&text, source)
}

/// Parse YAML text into a layer. Exposed for tests and for configuration
/// supplied inline rather than from a file.
pub fn layer_from_str(text: &str, source: Source) -> Result<Layer, Error> {
    let value: serde_json::Value = serde_saphyr::from_str(text).map_err(|e| Error::Syntax {
        source_label: source.label(),
        message: e.to_string(),
    })?;
    Ok(Layer::new(source, value))
}

/// Merge layers, deserialize, and validate.
///
/// Errors stop the load. Warnings are returned with the configuration, because
/// a configuration that is merely unwise is still the user's to make.
pub fn load(layers: Vec<Layer>) -> Result<Loaded, Error> {
    let merged = merge::merge(layers);

    let mut track = serde_path_to_error::Track::new();
    let de = serde_path_to_error::Deserializer::new(merged.value.clone(), &mut track);
    let config = Config::deserialize(de).map_err(|e| {
        let path = track.path().to_string();
        let origin = merged
            .origin(&path)
            .map(Source::label)
            .unwrap_or_else(|| "a default".to_string());
        Error::Shape {
            path: if path.is_empty() {
                "<root>".into()
            } else {
                path
            },
            message: e.to_string(),
            origin,
        }
    })?;

    let problems = validate::check(&config);
    if validate::errors(&problems).next().is_some() {
        return Err(Error::Invalid(problems));
    }

    Ok(Loaded {
        config,
        merged,
        warnings: problems,
    })
}

/// Default location of the global user config.
pub fn global_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
    Some(base.join("corifeu").join("config.yaml"))
}

/// Default location of the project config, relative to a repository root.
pub fn project_config_path(root: &Path) -> PathBuf {
    root.join(".corifeu").join("config.yaml")
}
