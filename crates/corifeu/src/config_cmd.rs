//! `corifeu config` — inspecting the effective configuration.

use corifeu_config::{Layer, Source, env, explain, layer_from_file, load};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn show(explain_sources: bool, file: Option<&Path>) -> ExitCode {
    let layers = match collect_layers(file) {
        Ok(layers) => layers,
        Err(code) => return code,
    };

    if layers.iter().all(|l| matches!(l.source, Source::Env(_))) {
        eprintln!("error: no configuration found.");
        eprintln!();
        eprintln!("Looked for:");
        if let Some(path) = project_path() {
            eprintln!("  project  {}", path.display());
        }
        if let Some(path) = corifeu_config::global_config_path() {
            eprintln!("  global   {}", path.display());
        }
        eprintln!();
        eprintln!("Pass --file to read one directly.");
        return ExitCode::FAILURE;
    }

    let loaded = match load(layers) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    for warning in &loaded.warnings {
        eprintln!("{warning}");
    }
    if !loaded.warnings.is_empty() {
        eprintln!();
    }

    if explain_sources {
        match loaded.entries() {
            Ok(entries) => print!("{}", explain::render_table(&entries)),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        match serde_json::to_string_pretty(&loaded.config) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

/// Layers, weakest first. Missing files are not an error: a user may have only
/// a project config, or only a global one.
pub(crate) fn collect_layers(file: Option<&Path>) -> Result<Vec<Layer>, ExitCode> {
    let mut layers = Vec::new();

    if let Some(path) = corifeu_config::global_config_path()
        && path.exists()
    {
        match layer_from_file(&path, Source::Global(path.clone())) {
            Ok(layer) => layers.push(layer),
            Err(e) => {
                eprintln!("error: {e}");
                return Err(ExitCode::FAILURE);
            }
        }
    }

    let project = file.map(Path::to_path_buf).or_else(project_path);
    if let Some(path) = project {
        if path.exists() {
            match layer_from_file(&path, Source::Project(path.clone())) {
                Ok(layer) => layers.push(layer),
                Err(e) => {
                    eprintln!("error: {e}");
                    return Err(ExitCode::FAILURE);
                }
            }
        } else if file.is_some() {
            eprintln!("error: {}: no such file", path.display());
            return Err(ExitCode::FAILURE);
        }
    }

    layers.extend(env::layers());
    Ok(layers)
}

pub(crate) fn project_path() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|cwd| corifeu_config::project_config_path(&cwd))
}
