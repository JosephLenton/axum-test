<div align="center">
  <h1>
    Axum Test<br>
    for testing Axum Servers
  </h1>

  [![crate](https://img.shields.io/crates/v/axum-test.svg)](https://crates.io/crates/axum-test)
  [![docs](https://docs.rs/axum-test/badge.svg)](https://docs.rs/axum-test)
</div>

This is a project to make it easier to test applications built using Axum.
Using full E2E tests.

Some of the design decisions behind this are very opinionated,
to encourage one to write good tests.

## Features

This is for spinning up an Axum service, that you can then query directly.
This is primarily for testing Axum services.

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

      assert_eq!(response.contents, "pong!");
  }
```

### Runs on a random port, allowing multiple to run at once

When you start the server, you can spin it up on a random port.
It allows multiple E2E tests to run in parallel, each on their own webserver.

This behaviour can be changed in the `TestServerConfig`, by selecting a custom ip or port to always use.

### Remembers cookies across requests

It is common in E2E tests that step 1 is to login, and step 2 is the main request.
To make this easier cookies returned from the server will be preserved,
and then included into the next request. Like a web browser.

### Fails fast on unexpected requests

By default; all requests will panic if the server fails to return a 2xx status code.
This [can be switched](https://docs.rs/axum-test/latest/axum_test/struct.TestRequest.html#method.expect_failure) to panic when the server _doesn't_ return a 200.

This is a very opinionated design choice, and is done to help test writers fail fast when writing tests.
