<div align="center">
  <h1>
    Example Snapshots<br/>
  </h1>

  <h3>
    Examples of using Insta with Axum Test for snapshotting responses
  </h3>

  <br/>
</div>

This is a series of tests to show how to use Axum Test with [Insta](https://crates.io/crates/insta).

In the subfolder `/snapshots` you'll find the saved snapshots of
tests. Looking at them can show you what the snapshot output looks
like.

It is a tiny mock application that offers endpoints for:
 - `/todo/json`
 - `/todo/yaml`
 - `/example.png`

To run you can use the following command:
```bash
# Tests
cargo test --example=snapshots --features yaml

# To run the example
cargo run --example=snapshots --features yaml
```
