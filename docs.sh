#!/bin/bash

set -e

cargo +stable doc --features=all --open
