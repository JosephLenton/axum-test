<div align="center">
  <h1>
    Axum Test
  </h1>

  <h3>
    Easy E2E testing for applications built on Axum
  </h3>

  [![crate](https://img.shields.io/crates/v/axum-test.svg)](https://crates.io/crates/axum-test)
  [![docs](https://docs.rs/axum-test/badge.svg)](https://docs.rs/axum-test)

  <br/>
</div>

Using this library, you can host your application and query against it with requests. Then decode the responses, and assert what is returned.

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

### Axum Compatability

Axum Test requires the latest version of Axum (0.7).

| Axum Version | Axum Test Version |
|--------------|-------------------|
| 0.7 (latest) | 14, 15+ (latest)  |
| 0.6          | [13.4.1](https://crates.io/crates/axum-test/13.4.1)            |

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
 - Prettifying the assertion output.

## Crate Features

Here are a list of all features so far that can be enabled:

 * `all` _off by default_, turns on all features below.
 * `pretty-assertions` **on by default**, uses the [pretty assertions crate](https://crates.io/crates/pretty_assertions) for the output to the `assert_*` functions.
 * `yaml` _off by default_, adds support for sending, receiving, and asserting, [yaml content](https://yaml.org/).
 * `msgpack` _off by default_, adds support for sending, receiving, and asserting, [msgpack content](https://msgpack.org/index.html).
 * `axum-extra` _off by default_, adds support for the `TypedPath` from [axum-extra](https://crates.io/crates/axum-extra).

## Contributions

A big thanks to all of these who have helped!

<a href="https://github.com/josephlenton/axum-test/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=josephlenton/axum-test" />
</a>

Made with [contrib.rocks](https://contrib.rocks).
