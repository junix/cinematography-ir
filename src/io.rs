use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::model::CineProject;

pub fn load_project(path: impl AsRef<Path>) -> Result<CineProject> {
    let path = path.as_ref();
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

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

/// Serializes `project` to pretty JSON and publishes it atomically: the bytes
/// land in a same-directory temporary file that is renamed onto `path` only
/// after a successful write and flush. A failed or interrupted save never
/// destroys previous contents. This also defines in-place normalization
/// (`normalize input -o input`): the input is fully loaded before the swap,
/// so the output path always holds either the old or the new document.
pub fn save_project_json(project: &CineProject, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let text = serde_json::to_string_pretty(project).context("failed to serialize project")?;
    atomic_write(path, &format!("{text}\n"))
}

/// Write-then-rename within the target directory so `path` never exposes a
/// partial document and a same-filesystem rename publishes it atomically.
fn atomic_write(path: &Path, bytes: &str) -> Result<()> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path.file_name().map_or_else(
        || "project.json".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let temp = dir.join(format!(".{name}.tmp-{}", std::process::id()));

    let outcome = publish(&temp, path, bytes);
    if outcome.is_err() {
        // Never leave the temporary file behind on failure.
        let _ = fs::remove_file(&temp);
    }
    outcome
}

fn publish(temp: &Path, path: &Path, bytes: &str) -> Result<()> {
    let mut file =
        File::create(temp).with_context(|| format!("failed to write {}", temp.display()))?;
    file.write_all(bytes.as_bytes())
        .and_then(|()| file.sync_all())
        .with_context(|| format!("failed to write {}", temp.display()))?;
    // Preserve the replaced file's permissions so re-saving keeps modes such
    // as a restrictive 0600 untouched.
    if let Ok(previous) = fs::metadata(path) {
        fs::set_permissions(temp, previous.permissions())
            .with_context(|| format!("failed to apply permissions to {}", temp.display()))?;
    }
    fs::rename(temp, path).with_context(|| format!("failed to publish {}", path.display()))
}

#[cfg(test)]
#[path = "io_tests.rs"]
mod tests;
