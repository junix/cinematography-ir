.PHONY: fmt check test schema book examples snapshots

fmt:
	cargo fmt --all

check:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

schema:
	cargo run -- schema --output schema/cinematography-ir.schema.json
	cargo run -- schema --solved --output schema/solved-camera-ir.schema.json
	cargo run -- schema --estimated --output schema/estimated-trajectory.schema.json

snapshots:
	UPDATE_SNAPSHOTS=1 cargo test --test view ascii_plan_snapshots_are_stable

book:
	mdbook build

examples:
	cargo run -- validate examples/dialogue.yaml --deny-warnings
	cargo run -- validate examples/intentional_axis_cross.yaml --deny-warnings
	cargo run -- validate examples/dolly_zoom.yaml --deny-warnings
	cargo run -- analyze examples/unsafe_axis_cross.yaml
	cargo run -- solve examples/dolly_zoom.yaml --deny-warnings -o /dev/null
	cargo run -- prompt examples/dialogue.yaml > /dev/null
	cargo run -- view examples/dialogue.yaml --format ascii --layout strip:v > /dev/null
	cargo run -- render blender examples/dialogue.yaml --out-dir target/examples/passes --script-only > /dev/null
