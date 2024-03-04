#!/bin/bash

set -e

cargo check
cargo test --example=example-todo
cargo test  --features yaml,msgpack,pretty-assertions "$@"
cargo test "$@"
