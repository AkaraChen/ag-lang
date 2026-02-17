Commit after opsx:apply

## Rules

- NEVER run multiple `cargo build`/`cargo run`/`cargo test` commands in background simultaneously — parallel Rust compilations will exhaust system memory and crash the machine. Always run them sequentially.