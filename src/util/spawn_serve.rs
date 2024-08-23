use ::axum::extract::Request;
use ::axum::response::Response;
use ::axum::serve;
use ::axum::serve::IncomingStream;
use ::std::convert::Infallible;
use ::tokio::net::TcpListener;
use ::tokio::spawn;
use ::tower::Service;

use crate::util::ServeHandle;

/// A higher lever alternative to [`axum::serve`] for tests.
///
/// This spawns in a new thread, and adds a shutdown handle the service
/// that is attached to the returned [`crate::util::ServeHandle`].
///
/// When the handle is dropped, the service will be terminated.
pub fn spawn_serve<M, S>(tcp_listener: TcpListener, make_service: M) -> ServeHandle
where
    M: for<'a> Service<IncomingStream<'a>, Error = Infallible, Response = S> + Send + 'static,
    for<'a> <M as Service<IncomingStream<'a>>>::Future: Send,
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    let server_handle = spawn(async move {
        serve(tcp_listener, make_service)
            .await
            .expect("Expect server to start serving");
    });

    ServeHandle::new(server_handle)
}
