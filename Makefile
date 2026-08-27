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
	mkdir -p target/examples
	cargo run -- validate examples/dialogue.yaml --deny-warnings
	cargo run -- validate examples/intentional_axis_cross.yaml --deny-warnings
	cargo run -- validate examples/dolly_zoom.yaml --deny-warnings
	cargo run -- validate examples/jaws_beach_dolly_zoom.yaml --deny-warnings
	cargo run -- validate examples/follow_handheld.yaml --deny-warnings
	cargo run -- validate examples/crane_reveal.yaml --deny-warnings
	cargo run -- analyze examples/unsafe_axis_cross.yaml
	cargo run -- solve examples/dolly_zoom.yaml --deny-warnings -o /dev/null
	cargo run -- solve examples/jaws_beach_dolly_zoom.yaml --deny-warnings -o /dev/null
	cargo run -- solve examples/follow_handheld.yaml --deny-warnings -o /dev/null
	cargo run -- solve examples/crane_reveal.yaml --deny-warnings -o /dev/null
	cargo run -- prompt examples/dialogue.yaml > /dev/null
	cargo run -- view examples/dialogue.yaml --format ascii --layout strip:v > /dev/null
	cargo run -- view examples/jaws_beach_dolly_zoom.yaml --layout animate:12 --format html -o target/examples/jaws-beach.html
	cargo run -- view examples/follow_handheld.yaml --layout animate:12 --format html -o target/examples/follow-handheld.html
	cargo run -- view examples/crane_reveal.yaml --format png -o target/examples/crane-reveal.png
	cargo run -- render blender examples/dialogue.yaml --out-dir target/examples/passes --script-only > /dev/null
	cargo run -- render blender examples/follow_handheld.yaml --out-dir target/examples/follow-passes --passes depth,normal,id --script-only > /dev/null
	cargo run -- render blender examples/crane_reveal.yaml --out-dir target/examples/reveal-passes --passes depth,normal,id,openpose --script-only > /dev/null
