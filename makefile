.PHONY: fmt lint test build publish docs codecov

fmt:
	cargo fmt

lint:
	cargo +stable clippy

test:
	cargo +stable check
	cargo +stable test --example=todo
	cargo +stable test --example=snapshots --features yaml
	cargo +stable test --example=websocket-ping-pong --features ws
	cargo +stable test --example=websocket-chat --features ws
	cargo +stable test  --features all
	cargo +stable test

	# Check deprecated old-json-diff still works
	cargo +stable test  --features "old-json-diff"
	cargo +stable test  --features "ws,old-json-diff"

	# Check minimum version works
	cargo +1.85 check --features "pretty-assertions,yaml,msgpack,reqwest,typed-routing,ws"

	# Check nightly also works, see https://github.com/JosephLenton/axum-test/issues/133
	cargo +nightly check --features all

	# Check the various build variations work
	cargo +stable check --no-default-features
	cargo +stable check --features all
	cargo +stable check --features pretty-assertions
	cargo +stable check --features yaml
	cargo +stable check --features msgpack
	cargo +stable check --features reqwest
	cargo +stable check --features typed-routing
	cargo +stable check --features ws
	cargo +stable check --features "ws,old-json-diff"
	cargo +stable check --features reqwest
	cargo +stable check --features old-json-diff

	cargo +stable clippy --features all

build:
	cargo +stable build

publish: fmt lint test
	cargo publish

docs:
	cargo doc --open --features all

codecov:
	cargo llvm-cov --open
