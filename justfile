# cinematography-ir — build / test / install (workspace convention, see ~/projects/CLAUDE.md)

os_name := if os() == "macos" { "macos" } else { "linux" }
arch_name := if arch() == "aarch64" { "arm64" } else { "x86" }
default_install_bin := home_directory() / "sync" / (os_name + "-" + arch_name + "-bin")
install_bin := env("SYNC_BIN_DIR", default_install_bin)

default: build

build:
    cargo build --release

test:
    cargo test --all

check:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Validate, solve, prompt, and render every example (no Blender needed).
examples:
    cargo run -q -- validate examples/dialogue.yaml --deny-warnings
    cargo run -q -- validate examples/intentional_axis_cross.yaml --deny-warnings
    cargo run -q -- validate examples/dolly_zoom.yaml --deny-warnings
    cargo run -q -- analyze examples/unsafe_axis_cross.yaml
    cargo run -q -- solve examples/dolly_zoom.yaml --deny-warnings -o /dev/null
    cargo run -q -- prompt examples/dialogue.yaml > /dev/null
    cargo run -q -- view examples/dialogue.yaml --format ascii --layout strip:v > /dev/null
    cargo run -q -- render blender examples/dialogue.yaml --out-dir target/examples/passes --script-only > /dev/null

schema:
    cargo run -q -- schema --output schema/cinematography-ir.schema.json
    cargo run -q -- schema --solved --output schema/solved-camera-ir.schema.json
    cargo run -q -- schema --estimated --output schema/estimated-trajectory.schema.json

# Refresh ASCII view snapshots after an intentional rendering change.
snapshots:
    UPDATE_SNAPSHOTS=1 cargo test --test view ascii_plan_snapshots_are_stable

install: build
    mkdir -p "{{ install_bin }}"
    cp target/release/cine-ir "{{ install_bin }}/cine-ir"
