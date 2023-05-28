use ::axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::hyper::server::conn::AddrIncoming;
use ::hyper::server::conn::AddrStream;
use ::hyper::server::Builder;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

/// This exists to gloss over the differences between Axum's
/// `IntoMakeService` and `IntoMakeServiceWithConnectInfo` types.
/// In theory it also allows others as well.
///
/// This is a trait for turning those types into a thread, that is
/// running a web server. The server should be built using the `Builder`
/// provided.
pub trait IntoTestServerThread {
    fn into_server_thread(self, server_builder: Builder<AddrIncoming>) -> JoinHandle<()>;
}

impl IntoTestServerThread for IntoMakeService<Router> {
    fn into_server_thread(self, server_builder: Builder<AddrIncoming>) -> JoinHandle<()> {
        let server = server_builder.serve(self);
        spawn(async move {
            server.await.expect("Expect server to start serving");
        })
    }
}

impl<C> IntoTestServerThread for IntoMakeServiceWithConnectInfo<Router, C>
where
    for<'a> C: axum::extract::connect_info::Connected<&'a AddrStream>,
{
    fn into_server_thread(self, server_builder: Builder<AddrIncoming>) -> JoinHandle<()> {
        let server = server_builder.serve(self);
        spawn(async move {
            server.await.expect("Expect server to start serving");
        })
    }
}

#[cfg(test)]
mod test_IntoTestServerThread_for_IntoMakeService {
    use ::axum::extract::State;
    use ::axum::routing::get;
    use ::axum::Router;

    use crate::TestServer;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    async fn get_state(State(count): State<u32>) -> String {
        format!("count is {}", count)
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service_with_state() {
        // Build an application with a route.
        let app = Router::new()
            .route("/count", get(get_state))
            .with_state(123)
            .into_make_service();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/count").await.assert_text(&"count is 123");
    }
}

#[cfg(test)]
mod test_IntoTestServerThread_for_IntoMakeServiceWithConnectInfo {
    use ::axum::routing::get;
    use ::axum::Router;
    use ::std::net::SocketAddr;

    use crate::TestServer;

    async fn get_ping() -> &'static str {
        "pong!"
    }

    #[tokio::test]
    async fn it_should_create_and_test_with_make_into_service_with_connect_info() {
        // Build an application with a route.
        let app = Router::new()
            .route("/ping", get(get_ping))
            .into_make_service_with_connect_info::<SocketAddr>();

        // Run the server.
        let server = TestServer::new(app).expect("Should create test server");

        // Get the request.
        server.get(&"/ping").await.assert_text(&"pong!");
    }
}
