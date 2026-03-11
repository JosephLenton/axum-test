use crate::util::ServeHandle;
use axum::extract::Request;
use axum::response::Response;
use axum::serve;
use axum::serve::IncomingStream;
use axum::serve::Listener;
use core::fmt::Debug;
use std::convert::Infallible;
use tokio::spawn;
use tower::Service;

#[cfg(feature = "actix-web")]
use actix_web::App;
#[cfg(feature = "actix-web")]
use actix_web::HttpServer;
#[cfg(feature = "actix-web")]
use actix_web::body::MessageBody;
#[cfg(feature = "actix-web")]
use actix_web::dev::ServiceFactory;
#[cfg(feature = "actix-web")]
use actix_web::dev::ServiceRequest;
#[cfg(feature = "actix-web")]
use actix_web::dev::ServiceResponse;
#[cfg(feature = "actix-web")]
use actix_web::rt::System as ActixRtSystem;
#[cfg(feature = "actix-web")]
use tokio::net::TcpListener;

/// A wrapper around [`axum::serve()`] for tests,
/// which spawns the service in a new thread.
///
/// The [`crate::util::ServeHandle`] returned will automatically attempt
/// to terminate the service when dropped.
pub fn spawn_serve<L, M, S>(tcp_listener: L, make_service: M) -> ServeHandle
where
    L: Listener,
    L::Addr: Debug,
    M: for<'a> Service<IncomingStream<'a, L>, Error = Infallible, Response = S> + Send + 'static,
    for<'a> <M as Service<IncomingStream<'a, L>>>::Future: Send,
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

/// A wrapper around [`actix_web::HttpServer`] for tests,
/// which spawns the service in a new thread.
///
/// The [`crate::util::ServeHandle`] returned will automatically attempt
/// to terminate the service when dropped.
#[cfg(feature = "actix-web")]
pub fn spawn_actix_serve<F, T, B>(tcp_listener: TcpListener, actix_web_app: F) -> ServeHandle
where
    // F: Fn() -> App<T> + Send + Clone + 'static,
    F: Fn() -> App<T> + Send + Clone + 'static,
    T: ServiceFactory<
            ServiceRequest,
            Config = (),
            Response = ServiceResponse<B>,
            Error = actix_web::Error,
            InitError = (),
        > + 'static,
    B: MessageBody + 'static,
{
    let join_handle = spawn(async move {
        ActixRtSystem::new().block_on(async move {
            let std_tcp_listener = tcp_listener
                .into_std()
                .expect("Failed to turn tokio TcpListener into std TcpListener");

            HttpServer::new(actix_web_app)
                .listen(std_tcp_listener)
                .expect("Failed to bind actix-web server to listener")
                .run()
                .await
                .expect("Actix-web server encountered an error");
        });
    });

    ServeHandle::new(join_handle)
}
