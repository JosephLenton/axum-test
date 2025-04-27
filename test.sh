#!/bin/bash

set -e

cargo +stable check
cargo +stable test --example=example-shuttle --features shuttle
cargo +stable test --example=example-todo
cargo +stable test --example=example-websocket-ping-pong --features ws
cargo +stable test --example=example-websocket-chat --features ws
cargo +stable test  --features all "$@"
cargo +stable test "$@"

# Check minimum version works, excluding shuttle
cargo +1.83 check --features "pretty-assertions,yaml,msgpack,reqwest,typed-routing,ws"
# Check nightly also works, see https://github.com/JosephLenton/axum-test/issues/133
cargo +nightly check --features all "$@"

# Check the various build variations work
cargo +stable check --no-default-features
cargo +stable check --features all
cargo +stable check --features pretty-assertions
cargo +stable check --features yaml
cargo +stable check --features msgpack
cargo +stable check --features reqwest
cargo +stable check --features shuttle
cargo +stable check --features typed-routing
cargo +stable check --features ws
cargo +stable check --features reqwest
cargo +stable check --features old-json-diff

cargo +stable clippy --features all
