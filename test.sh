#!/bin/bash

set -e

cargo check
cargo test --example=example-todo
cargo test --example=example-websocket-ping-pong --features ws
cargo test --example=example-websocket-chat --features ws
cargo test  --features all "$@"
cargo test "$@"

# Check the various build variations work
cargo check --no-default-features
cargo check --features all
cargo check --features pretty-assertions
cargo check --features yaml
cargo check --features msgpack
cargo check --features typed-routing
cargo check --features ws
