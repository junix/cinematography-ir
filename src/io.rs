use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::model::CineProject;

pub fn load_project(path: impl AsRef<Path>) -> Result<CineProject> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("yaml" | "yml") => serde_yaml::from_str(&text)
            .with_context(|| format!("invalid YAML in {}", path.display())),
        Some("json") => serde_json::from_str(&text)
            .with_context(|| format!("invalid JSON in {}", path.display())),
        Some(other) => bail!("unsupported extension .{other}; expected .yaml, .yml, or .json"),
        None => bail!("input file has no extension; expected .yaml, .yml, or .json"),
    }
}

pub fn save_project_json(project: &CineProject, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let text = serde_json::to_string_pretty(project).context("failed to serialize project")?;
    fs::write(path, format!("{text}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}
