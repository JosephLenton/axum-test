<div align="center">
  <h1>
    Axum Test<br>
    for testing Axum Servers
  </h1>

  [![crate](https://img.shields.io/crates/v/axum-test.svg)](https://crates.io/crates/axum-test)
  [![docs](https://docs.rs/axum-test/badge.svg)](https://docs.rs/axum-test)
</div>

This is a project to make it easier to test applications built using Axum,
using full E2E tests.

## Examples

You can find a thorough example in the [/examples folder](/examples/example-todo/).

## Features

You can start a webserver running your application, query it with requests,
and then assert the responses returned:

```rust
  use ::axum::Router;
  use ::axum::routing::get;

  use ::axum_test_server::TestServer;

  async fn get_ping() -> &'static str {
      "pong!"
  }

  #[tokio::test]
  async fn it_should_get() {
      // Build an application with a route.
      let app = Router::new()
          .route("/ping", get(get_ping))
          .into_make_service();

      // Run the server on a random address.
      let server = TestServer::new(app).unwrap();

      // Get the request.
      let response = server
          .get("/ping")
          .await;

      assert_eq!(response.as_bytes(), "pong!");
  }
```

The `TestServer` will start on a random port, allowing multiple servers to run in parallel.
That allows tests to be run in parallel.

This behaviour can be changed in the `TestServerConfig`, by selecting a custom ip or port to always use.

### Request building

Querying your application on the `TestServer` supports all of the common request building you would expect.

 - Serlializing and deserializing Json and Form content using Serde
 - Cookie setting and reading
 - Access to setting and reading headers
 - Status code reading and assertions
 - Assertions for defining what you expect to have returned

### Remembers cookies across requests

`axum-test` supports preserving Cookies from responses,
for use in follow up requests to same `TestServer`.
This is similar to how a browser will store cookies from requests,
and can help with tests where you need to authenticate first.

This is _off_ by default, and can be enabled in the `TestServerConfig` by setting `save_cookies` to true.
