use ::axum::extract::Request;
use ::axum::response::Response;
use ::axum::serve;
use ::axum::serve::IncomingStream;
use ::std::convert::Infallible;
use ::std::future::Future;
use ::std::sync::Arc;
use ::tokio::net::TcpListener;
use ::tokio::spawn;
use ::tokio::sync::Notify;
use ::tower::Service;

use crate::util::ServeHandle;

pub fn spawn_serve<M, S>(tcp_listener: TcpListener, make_service: M) -> ServeHandle
where
    M: for<'a> Service<IncomingStream<'a>, Error = Infallible, Response = S> + Send + 'static,
    for<'a> <M as Service<IncomingStream<'a>>>::Future: Send,
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    let shutdown_notification = Arc::new(Notify::new());
    let shutdown_signal = shutdown_future(Arc::clone(&shutdown_notification));

    let server_handle = spawn(async move {
        serve(tcp_listener, make_service)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .expect("Expect server to start serving");
    });

    ServeHandle::new(server_handle, shutdown_notification)
}

fn shutdown_future(
    shutdown_notification_listener: Arc<Notify>,
) -> impl Future<Output = ()> + Send + 'static {
    async move { shutdown_notification_listener.notified().await }
}
