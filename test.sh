#!/bin/bash

set -e

cargo check
cargo test --example=example-todo
cargo test  --features all "$@"
cargo test "$@"

cargo check --features all
cargo check --features msgpack
cargo check --features yaml
cargo check --features typed-routing
cargo check --features ws
