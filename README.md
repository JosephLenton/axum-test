<div align="center">
  <h1>
    Axum Test<br>
    for testing Axum Servers
  </h1>

  [![crate](https://img.shields.io/crates/v/axum-test.svg)](https://crates.io/crates/axum-test)
  [![docs](https://docs.rs/axum-test/badge.svg)](https://docs.rs/axum-test)
</div>

Easy E2E testing for applications built on Axum.

Using this library, you can host your application and query against it with requests.
Then decode the responses, and assert what is returned:

```rust
  use ::axum::Router;
  use ::axum::routing::get;

  use ::axum_test::TestServer;

  async fn get_ping() -> &'static str {
      "pong!"
  }

  #[tokio::test]
  async fn it_should_get() {
      // Build an application with a route.
      let app = Router::new()
          .route("/ping", get(get_ping));

      // Run the application for testing.
      let server = TestServer::new(app).unwrap();

      // Get the request.
      let response = server
          .get("/ping")
          .await;

      assert_eq!(response.text(), "pong!");
  }
```

The `TestServer` can run requests directly against your application with a mocked network,
or the application can run on a random port (with real network reqeusts being made).
In both cases allowing multiple servers to run in parallel, across your tests.

This behaviour can be changed in the `TestServerConfig`, by selecting the `transport` to be used.

### Example

You can find a thorough example in the [/examples folder](/examples/example-todo/).

### Request building

Querying your application on the `TestServer` supports all of the common request building you would expect.

 - Serializing and deserializing Json and Form content using Serde
 - Cookie setting and reading
 - Access to setting and reading headers
 - Status code reading and assertions
 - Assertions for defining what you expect to have returned

### It also includes

 - Saving cookies returned for use across future requests.
 - Setting headers and query parameters for use across all TestRequests.
 - Can optionally run requests using a real web server.
 - Automatic status assertions for checking requests always succeed or fail.
