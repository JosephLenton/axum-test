#!/bin/bash

set -e

cargo check
cargo test --example=example-todo
cargo test  --features yaml,pretty-assertions "$@"
cargo test "$@"
