.PHONY: fmt check test schema book examples

fmt:
	cargo fmt --all

check:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all

schema:
	cargo run -- schema --output schema/cinematography-ir.schema.json

book:
	mdbook build

examples:
	cargo run -- validate examples/dialogue.yaml --deny-warnings
	cargo run -- validate examples/intentional_axis_cross.yaml --deny-warnings
	cargo run -- validate examples/dolly_zoom.yaml --deny-warnings
	cargo run -- analyze examples/unsafe_axis_cross.yaml
