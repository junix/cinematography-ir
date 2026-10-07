use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

fn scratch_dir(label: &str) -> PathBuf {
    let unique = format!(
        "cine-ir-io-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

fn example_project() -> CineProject {
    load_project(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/dialogue.yaml"))
        .expect("example project must parse")
}

fn directory_entries(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .expect("read scratch directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn save_writes_valid_complete_json() {
    let dir = scratch_dir("normal");
    let target = dir.join("normalized.json");
    let project = example_project();

    save_project_json(&project, &target).expect("save");

    let text = fs::read_to_string(&target).expect("read back");
    assert!(
        text.ends_with("}\n"),
        "document must end with a closing brace and newline"
    );
    let reloaded = load_project(&target).expect("normalized JSON must reload");
    assert_eq!(reloaded, project);
    assert_eq!(
        directory_entries(&dir),
        vec!["normalized.json".to_owned()],
        "no temporary file may survive a successful save"
    );
    fs::remove_dir_all(&dir).expect("clean up scratch directory");
}

#[test]
fn in_place_save_swaps_atomically() {
    let dir = scratch_dir("in-place");
    let target = dir.join("project.json");
    let mut project = example_project();
    project.title = "First Pass".to_owned();
    save_project_json(&project, &target).expect("initial save");

    // In-place normalization: load, modify, save back to the same path.
    let mut reloaded = load_project(&target).expect("reload");
    reloaded.title = "Second Pass".to_owned();
    save_project_json(&reloaded, &target).expect("in-place save");

    let final_project = load_project(&target).expect("final reload");
    assert_eq!(final_project.title, "Second Pass");
    let text = fs::read_to_string(&target).expect("read back");
    assert!(
        text.ends_with("}\n"),
        "in-place document must stay complete"
    );
    assert_eq!(
        directory_entries(&dir),
        vec!["project.json".to_owned()],
        "no temporary file may survive an in-place save"
    );
    fs::remove_dir_all(&dir).expect("clean up scratch directory");
}

#[cfg(unix)]
#[test]
fn failed_write_keeps_previous_bytes() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch_dir("readonly");
    let target = dir.join("project.json");
    let project = example_project();
    save_project_json(&project, &target).expect("seed save");
    let before = fs::read(&target).expect("seed bytes");

    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).expect("lock directory");
    let failed = save_project_json(&project, &target);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("unlock directory");

    assert!(
        failed.is_err(),
        "writing into a read-only directory must fail"
    );
    assert_eq!(
        fs::read(&target).expect("bytes after failure"),
        before,
        "a failed write must leave the old document intact"
    );
    assert_eq!(
        directory_entries(&dir),
        vec!["project.json".to_owned()],
        "a failed write must clean up its temporary file"
    );
    fs::remove_dir_all(&dir).expect("clean up scratch directory");
}

#[cfg(unix)]
#[test]
fn save_preserves_existing_file_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch_dir("mode");
    let target = dir.join("project.json");
    save_project_json(&example_project(), &target).expect("seed save");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).expect("restrict mode");

    save_project_json(&example_project(), &target).expect("re-save");

    let mode = fs::metadata(&target)
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "re-saving must keep the previous mode");
    fs::remove_dir_all(&dir).expect("clean up scratch directory");
}
