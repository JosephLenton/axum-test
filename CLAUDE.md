# axum-test — Agent Guidelines

## Project Overview

`axum-test` is a Rust library for easy end-to-end (E2E) testing of Axum web applications. It supports REST APIs, WebSockets, and multiple content types (JSON, YAML, MessagePack, multipart forms).

- **Rust Edition:** 2024
- **MSRV:** 1.85
- **Toolchain:** stable

## Build & Test Commands

```bash
# Format code
cargo fmt

# Lint
cargo +stable clippy

# Run all tests (preferred — mirrors CI)
make test

# Quick test (default features)
cargo +stable test

# Test with all features
cargo +stable test --features all

# Run specific example tests
cargo +stable test --example=todo
cargo +stable test --example=snapshots --features yaml
cargo +stable test --example=websocket-ping-pong --features ws
cargo +stable test --example=websocket-chat --features ws

# Doc tests
cargo +stable test --features all --doc

# Build docs
cargo doc --open --features all
```

`make test` runs the full suite: check, all examples, feature combinations, clippy, MSRV check (1.85), and nightly compatibility check.

## Project Structure

- `src/` — library source; `src/lib.rs` is the entry point for navigation
- `examples/` — instructional examples (`todo`, `snapshots`, `websocket-ping-pong`, `websocket-chat`)
- `tests/` — integration tests
- `files/` — test fixture files (JSON, YAML, TXT)

## Features

New code must either be ungated (available to all users) or gated behind a specific feature flag. Do not add functionality that is always compiled in but only useful for one optional integration.

- Gate optional code with `#[cfg(feature = "my-feature")]`
- Add the optional dependency in `Cargo.toml` under `[dependencies]` with `optional = true`
- Add the feature to the `all` feature so it is included in `cargo test --features all`
- Gate example tests that require the feature with `--features <name>` (see `makefile`)

The existing features are `pretty-assertions` (default), `yaml`, `msgpack`, `typed-routing`, `ws`, and `reqwest` — see `Cargo.toml` for their definitions.

## Code Style & Conventions

- **Zero unsafe code** — `#![forbid(unsafe_code)]` is set in `lib.rs`
- **Error handling** — use `anyhow::Result<T>`; add context with `.context()`
- **Builder pattern** — used throughout for configuration structs
- **No `unwrap()` in library code** — propagate errors properly
- **Imports** — use full `use` imports at the top of the file; avoid inline path lookups (e.g. `some::Type`) in function bodies and signatures
- **Aliased std results** — use `use std::io::Result as IoResult` and `use std::fmt::Result as FmtResult` rather than writing `std::io::Result` or `std::fmt::Result` inline
- Doc comments on all public API items

## Testing Patterns

Tests are written inside source files (not separate test files). Each example has `#[cfg(test)]` modules at the bottom with `#[tokio::test]` async tests.

Tests follow a pattern of:
 - Unit tests go in the same file as the code, at the bottom of the code file, after the features code.
 - a module with convention of `test_<name of function being tested>`
 - a list of tests using the naming convention of `it_should_<do x>_<when_y_condition>`
 - test modules should start with `super::*` to reuse imports from the parent module.

Example pattern:
```rust
#[cfg(test)]
mod test_fmt {
    use super::*;

    #[test]
    fn it_should_format_range() {
        let output = StatusCodeRangeFormatter(StatusCode::OK..StatusCode::IM_USED).to_string();
        assert_eq!(output, "200..226");
    }
}
```

## Examples Patterns

Examples of using Axum Test exist in the `/examples` folder. These are expected to be read by external users, and are to be instructional for external users to follow.

The tests in `/examples` should be built as clean instructional examples for external users to copy and use as a guide.

## Contributing

- Work on a feature branch; the main branch is `main`
- Run `make test` before considering any change complete
- PRs are opened on GitHub by the user, not by Claude — do not open or merge PRs
- Never run `cargo publish` or `make publish`
- The PR template is at `.github/pull_request_template.md`

## Important Notes

- When adding a new public API item, add doc comments and a usage example
- `src/test_request.rs` and `src/test_response.rs` each exceed 3000 lines — read the full function signature and surrounding context before editing either file
