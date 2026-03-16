//!
//! This is a simple Server-Sent Events (SSE) example Application.
//! It demonstrates two SSE endpoints:
//!
//!  - GET /events ... returns a finite stream of 5 events then closes.
//!  - GET /infinite ... returns an infinite stream of events, one every 100ms.
//!
//! At the bottom of this file are a series of tests for using SSE.
//!
//! ```bash
//! # To run it's tests:
//! cargo test --example=server-side-events
//! ```
//!

use anyhow::Result;
use axum::Router;
use axum::response::sse::Event;
use axum::response::sse::Sse;
use axum::routing::get;
use axum::serve::serve;
use futures_util::stream;
use std::convert::Infallible;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use futures::Stream;
use tokio::time::sleep;

#[cfg(test)]
use axum_test::TestServer;

const PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    let result: Result<()> = {
        let app = new_app();

        // Start!
        let ip_address = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let address = SocketAddr::new(ip_address, PORT);
        let listener = TcpListener::bind(address).await.unwrap();
        serve(listener, app.into_make_service()).await.unwrap();

        Ok(())
    };

    match &result {
        Err(err) => eprintln!("{}", err),
        _ => {}
    };
}

pub async fn route_get_events() -> Sse<impl Stream<Item = Result<Event, Infallible>>>
{
    let events = vec![
        Event::default().data("event-1"),
        Event::default().data("event-2"),
        Event::default().data("event-3"),
        Event::default().data("event-4"),
        Event::default().data("event-5"),
    ];

    let event_stream = stream::iter(events.into_iter().map(Ok));

    Sse::new(event_stream)
}

pub async fn route_get_infinite() -> Sse<impl Stream<Item = Result<Event, Infallible>>>
{
    let event_stream = stream::unfold(0u64, |counter| async move {
        sleep(Duration::from_millis(100)).await;
        let event = Event::default().data(format!("event-{counter}"));
        Some((Ok(event), counter + 1))
    });

    Sse::new(event_stream)
}

pub(crate) fn new_app() -> Router {
    Router::new()
        .route("/events", get(route_get_events))
        .route("/infinite", get(route_get_infinite))
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    TestServer::new(new_app())
}

#[cfg(test)]
mod test_route_get_events {
    use super::*;

    #[tokio::test]
    async fn it_should_return_all_events() {
        let server = new_test_app();

        let response = server.get("/events").await;

        response.assert_status_ok();
        response.assert_text(
            "data: event-1

data: event-2

data: event-3

data: event-4

data: event-5

",
        );
    }
}
