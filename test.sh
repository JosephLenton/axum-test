#!/bin/bash

set -e

cargo check
cargo test --example=example-todo
cargo test --example=example-websockets --features ws
cargo test  --features all "$@"
cargo test "$@"

# Check the various build variations work
cargo check --features all
cargo check --features msgpack
cargo check --features yaml
cargo check --features typed-routing
cargo check --features ws
