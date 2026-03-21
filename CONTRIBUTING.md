# Contributing to RecallBench

## Adding a Dataset

1. Create `recallbench/src/datasets/yourformat.rs`
2. Implement parsing into `BenchmarkQuestion` structs
3. Add to `datasets/mod.rs` registry
4. Add unit tests with fixture data

## Adding a System Adapter

1. Create `adapters/recallbench-yoursystem/`
2. Implement the `MemorySystem` trait
3. Or configure via HTTP/subprocess adapter TOML

## Code Style

- `cargo fmt` before committing
- `cargo clippy` should pass
- Unit tests for all new logic
- Integration tests for adapters

## Testing

```bash
cargo test --workspace
cargo clippy --workspace
cargo fmt --check
```
