# Vanguard Logic

Independent Rust workspace for a deterministic digital design platform.

## Architecture rule

`domain-model` is the innermost crate. It must remain independent from UI,
rendering, persistence, simulation, operating systems, and external services.
New capabilities are added through explicit crates and typed boundaries.

## Quality gates

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
