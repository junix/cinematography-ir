# JSON Schema

Generate the authoritative schema from the Rust types:

```bash
cargo run -- schema --output schema/cinematography-ir.schema.json
```

Do not hand-edit the generated file. Change `src/model.rs` and regenerate it.
