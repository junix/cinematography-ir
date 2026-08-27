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
    cp target/release/cine-ir "{{ install_bin }}/cine-ir"
