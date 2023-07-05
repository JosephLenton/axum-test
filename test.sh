#!/bin/bash

cargo test --example=example-todo
cargo test "$@"
