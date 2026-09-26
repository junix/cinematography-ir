# cinematography-ir — build / test / install (workspace convention, see ~/projects/CLAUDE.md)

os_name := if os() == "macos" { "macos" } else { "linux" }
arch_name := if arch() == "aarch64" { "arm64" } else { "x86" }
default_install_bin := home_directory() / "sync" / (os_name + "-" + arch_name + "-bin")
install_bin := env("SYNC_BIN_DIR", default_install_bin)

# Git build stamp: short sha, suffixed with ".dirty" when the worktree is dirty (ADR-1168).
stamp := `git rev-parse --short HEAD` + `(git diff --quiet && git diff --cached --quiet) >/dev/null 2>&1 || printf .dirty`

default: build

build:
    PM_BUILD_SHA=g{{stamp}} cargo build --release

test:
    cargo test --all

check:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Rebuild the single-file guide from the mdBook chapter order.
guide:
    sh tools/build_guide.sh

# Validate, compile, prompt, view, and package every example (no Blender needed).
examples:
    mkdir -p target/examples
    cargo run -q -- validate examples/dialogue.yaml --deny-warnings
    cargo run -q -- validate examples/intentional_axis_cross.yaml --deny-warnings
    cargo run -q -- validate examples/dolly_zoom.yaml --deny-warnings
    cargo run -q -- validate examples/jaws_beach_dolly_zoom.yaml --deny-warnings
    cargo run -q -- validate examples/follow_handheld.yaml --deny-warnings
    cargo run -q -- validate examples/crane_reveal.yaml --deny-warnings
    cargo run -q -- analyze examples/unsafe_axis_cross.yaml
    cargo run -q -- solve examples/dolly_zoom.yaml --deny-warnings -o /dev/null
    cargo run -q -- solve examples/jaws_beach_dolly_zoom.yaml --deny-warnings -o /dev/null
    cargo run -q -- compile examples/jaws_beach_dolly_zoom.yaml --deny-warnings -o /dev/null
    cargo run -q -- solve examples/follow_handheld.yaml --deny-warnings -o /dev/null
    cargo run -q -- solve examples/crane_reveal.yaml --deny-warnings -o /dev/null
    cargo run -q -- prompt examples/dialogue.yaml > /dev/null
    cargo run -q -- view examples/dialogue.yaml --format ascii --layout strip:v > /dev/null
    cargo run -q -- view examples/jaws_beach_dolly_zoom.yaml --intent storyboard --sampling phase-keyframes --panel plan,frame,elevation,timeline,metrics --format svg -o target/examples/jaws-storyboard.svg
    cargo run -q -- view examples/dolly_zoom.yaml --intent model-control --cue-map --sampling op-endpoints --layout separate --format svg -o target/examples/model-control
    cargo run -q -- view examples/jaws_beach_dolly_zoom.yaml --layout animate:12 --format html -o target/examples/jaws-beach.html
    cargo run -q -- view examples/follow_handheld.yaml --layout animate:12 --format html -o target/examples/follow-handheld.html
    cargo run -q -- view examples/crane_reveal.yaml --format png -o target/examples/crane-reveal.png
    cargo run -q -- render blender examples/dialogue.yaml --profile profiles/execution/generic-dense.json --out-dir target/examples/passes --script-only > /dev/null
    cargo run -q -- render blender examples/follow_handheld.yaml --out-dir target/examples/follow-passes --passes depth,normal,id --script-only > /dev/null
    cargo run -q -- render blender examples/crane_reveal.yaml --out-dir target/examples/reveal-passes --passes depth,normal,id,openpose --script-only > /dev/null

schema:
    cargo run -q -- schema --output schema/cinematography-ir.schema.json
    cargo run -q -- schema --solved --output schema/solved-camera-ir.schema.json
    cargo run -q -- schema --compiled --output schema/compiled-guidance-ir.schema.json
    cargo run -q -- schema --estimated --output schema/estimated-trajectory.schema.json
    cargo run -q -- schema --execution-profile --output schema/execution-profile.schema.json

# Refresh ASCII view snapshots after an intentional rendering change.
snapshots:
    UPDATE_SNAPSHOTS=1 cargo test --test view ascii_plan_snapshots_are_stable

install: build
    mkdir -p "{{ install_bin }}"
    @set -eu; dest="{{ install_bin }}/cine-ir"; mkdir -p "$(dirname "$dest")"; tmp="$(mktemp "{{ install_bin }}/.cine-ir.XXXXXX")"; trap 'rm -f "$tmp"' EXIT; cp "target/release/cine-ir" "$tmp"; chmod 755 "$tmp"; if [ "$(uname -s)" = "Darwin" ]; then xattr -c "$tmp" 2>/dev/null || true; codesign --force --sign - "$tmp"; fi; mv -f "$tmp" "$dest"

# Remove local build caches and documentation intermediates.
clean: clean-artifacts
    cargo clean

# Remove untracked intermediates. Preview: CLEAN_DRY_RUN=1 just clean-artifacts.
clean-artifacts:
    #!/usr/bin/env python3
    import fnmatch
    import glob
    import os
    from pathlib import Path
    import shutil
    import subprocess

    root = Path(r"""{{justfile_directory()}}""")
    os.chdir(root)
    dry_run = os.environ.get("CLEAN_DRY_RUN") == "1"
    if not (root / ".git").exists():
        print("No Git checkout at project root; preserving files.")
        raise SystemExit(0)
    # Fail closed if Git cannot identify protected files, including in worktrees.
    tracked = set(os.fsdecode(p) for p in subprocess.check_output(
        ["git", "ls-files", "-z"]).split(b"\0") if p)
    protected = set(tracked)
    for name in tracked:
        protected.update(str(p) for p in Path(name).parents)
    def remove(path):
        name = path.as_posix()
        if name in protected or path.is_symlink():
            return
        if path.is_dir():
            for directory, children, files in os.walk(path, followlinks=False):
                if (Path(directory) / ".git").exists():
                    return
                if any(Path(n).suffix in {".tex", ".typ", ".blend", ".ipynb"} for n in files):
                    return
                children[:] = [n for n in children if not (Path(directory) / n).is_symlink()]
        print(("Would remove " if dry_run else "Removing ") + name)
        if not dry_run:
            shutil.rmtree(path) if path.is_dir() else path.unlink()

    # Project-specific build outputs. Keep dependencies, models and final media.
    build_dirs = ["target"]
    for pattern in build_dirs:
        if Path(pattern).is_absolute() or ".." in Path(pattern).parts or pattern in {"", "."}:
            raise SystemExit("Build cleanup paths must stay within the project")
        for name in glob.glob(pattern):
            path = Path(name)
            if path.exists() and not any(p.is_symlink() for p in [path, *path.parents]):
                remove(path)

    caches = {"__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache"}
    skip = {".git", ".hg", ".svn", "legacy", "node_modules", ".venv", "venv", "vendor", "third_party", "target", ".build", ".lake", "dist-newstyle", ".stack-work"}
    tex = ("*.aux", "*.fls", "*.fdb_latexmk", "*.synctex.gz", "*.nav", "*.snm", "*.vrb", "*.bcf", "*.run.xml", "*.toc", "*.lof", "*.lot")
    images = {".png", ".jpg", ".jpeg", ".webp"}
    def walk_error(error):
        raise error
    for current, dirs, files in os.walk(".", onerror=walk_error, followlinks=False):
        base = Path(current)
        for name in dirs[:]:
            path = base / name
            if path.is_symlink() or name in skip or (path / ".git").exists():
                dirs.remove(name)
            elif name in caches or name.endswith(".egg-info"):
                remove(path)
                dirs.remove(name)
            elif name in {".cache", ".render-cache"} and "docs" in path.parts:
                ignored = subprocess.run(["git", "check-ignore", "-q", "--", str(path)])
                if ignored.returncode not in (0, 1):
                    raise SystemExit(ignored.returncode)
                if ignored.returncode == 0:
                    remove(path)
                    dirs.remove(name)
        for name in files:
            path = base / name
            if path.as_posix() in tracked or path.is_symlink():
                continue
            is_tex = any(fnmatch.fnmatchcase(name, pattern) for pattern in tex)
            is_tex_log = path.suffix == ".log" and path.with_suffix(".tex").is_file()
            parts = path.parts
            in_docs = any(p in {"docs", "doc", "infographics"} or p.endswith("-explainer") for p in parts[:-1])
            inspection = name.endswith(".lint.png") or name.startswith("slice-") or any(p in {"crops", "inspect", "slices", "tiles", "sections"} for p in parts[:-1])
            # Only ignored inspection images qualify; finished render/evidence trees stay.
            is_image = in_docs and inspection and path.suffix.lower() in images
            if is_image:
                result = subprocess.run(["git", "check-ignore", "-q", "--", str(path)])
                if result.returncode not in (0, 1):
                    raise SystemExit(result.returncode)
                is_image = result.returncode == 0
            if is_tex or is_tex_log or is_image:
                remove(path)
