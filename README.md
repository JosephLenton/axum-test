<div align="center">
  <h1>
    Axum Test
  </h1>

  <h3>
    Easy E2E testing for Axum<br/>
    including REST, WebSockets, and more
  </h3>

  [![crate](https://img.shields.io/crates/v/axum-test.svg)](https://crates.io/crates/axum-test)
  [![docs](https://docs.rs/axum-test/badge.svg)](https://docs.rs/axum-test)

  <br/>
</div>

This runs your application locally, allowing you to query against it with requests.
Decode the responses, and assert what is returned.

```rust
use axum::Router;
use axum::routing::get;

use axum_test::TestServer;

#[tokio::test]
async fn it_should_ping_pong() {
    // Build an application with a route.
    let app = Router::new()
        .route(&"/ping", get(|| async { "pong!" }));

    // Run the application for testing.
    let server = TestServer::new(app).unwrap();

    // Get the request.
    let response = server
        .get("/ping")
        .await;

    // Assertions.
    response.assert_status_ok();
    response.assert_text("pong!");
}
```

A `TestServer` enables you to run an Axum service with a mocked network,
or on a random port with real network reqeusts.
In both cases allowing you to run multiple servers, across multiple tests, all in parallel.

## Crate Features

| Feature             | On by default     |                                                                                                                                   |
|---------------------|-------------------|-----------------------------------------------------------------------------------------------------------------------------------|
| `all`               | _off_             | Turns on all features.                                                                                                            |
| `pretty-assertions` | **on**            | Uses the [pretty assertions crate](https://crates.io/crates/pretty_assertions) on response `assert_*` methods.                    |
| `yaml`              | _off_             | Enables support for sending, receiving, and asserting, [yaml content](https://yaml.org/).                                         |
| `msgpack`           | _off_             | Enables support for sending, receiving, and asserting, [msgpack content](https://msgpack.org/index.html).                         |
| `shuttle`           | _off_             | Enables support for building a `TestServer` an [`shuttle_axum::AxumService`](https://docs.rs/shuttle-axum/latest/shuttle_axum/struct.AxumService.html), for use with [Shuttle.rs](https://shuttle.rs). |
| `typed-routing`     | _off_             | Enables support for using `TypedPath` in requests. See [axum-extra](https://crates.io/crates/axum-extra) for details.             |
| `ws`                | _off_             | Enables WebSocket support. See [TestWebSocket](https://docs.rs/axum-test/latest/axum_test/struct.TestWebSocket.html) for details. |
| `reqwest`           | _off_             | Enables the `TestServer` being able to create [Reqwest](https://docs.rs/axum-test/latest/axum_test/struct.TestWebSocket.html) requests for querying. |

## Axum Compatability

The current version of Axum Test requires at least Axum v0.7.6.

Here is a list of compatability with prior versions:

| Axum Version            | Axum Test Version |
|-------------------------|-------------------|
| 0.7.6 to 0.7.9 (latest) | 16+ (latest)      |
| 0.7.0 to 0.7.5          | 14, 15            |
| 0.6                     | 13.4.1            |

## Examples

You can find examples of writing tests in the [/examples folder](/examples/).
These include tests for:

 * [a simple REST Todo application](/examples/example-todo), and [the same using Shuttle](/examples/example-shuttle)
 * [a WebSocket ping pong application](/examples/example-websocket-ping-pong) which sends requests up and down
 * [a simple WebSocket chat application](/examples/example-websocket-chat)

## Request Building Features

Querying your application on the `TestServer` supports all of the common request building you would expect.

 - Serializing and deserializing Json, Form, Yaml, and others, using Serde
 - Assertions on the Json, text, Yaml, etc, that is returned.
 - Cookie, query, and header setting and reading
 - Status code reading and assertions

### Also includes

 - WebSockets testing support
 - Saving returned cookies for use on future requests
 - Setting headers, query, and cookies, globally for all requests or on per request basis
 - Can run requests using a real web server, or with mocked HTTP
 - Automatic status assertions for expecting requests to succeed (to help catch bugs in tests sooner)
 - Prettified assertion output
 - Typed Routing from Axum Extra
 - Reqwest integration

## Contributions

A big thanks to all of these who have helped!

<a href="https://github.com/josephlenton/axum-test/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=josephlenton/axum-test" />
</a>

Made with [contrib.rocks](https://contrib.rocks).
