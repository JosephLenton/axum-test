use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::axum::Server;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::hyper::http::HeaderValue;
use ::hyper::http::Method;
use ::std::net::SocketAddr;
use ::std::net::TcpListener;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

use crate::util::new_random_socket_addr;
use crate::TestRequest;

/// A means to run Axum applications within a server that you can query.
/// This is for writing tests.
#[derive(Debug)]
pub(crate) struct InnerTestServer {
    server_thread: JoinHandle<()>,
    server_address: String,
    cookies: CookieJar,
}

impl InnerTestServer {
    /// This will take the given app, and run it.
    /// It will be run on a randomly picked port.
    ///
    /// The webserver is then wrapped within a `TestServer`,
    /// and returned.
    pub(crate) fn new(app: IntoMakeService<Router>) -> Result<Self> {
        let addr = new_random_socket_addr().context("Cannot create socket address for use")?;
        let test_server = Self::new_with_address(app, addr).context("Cannot create TestServer")?;

        Ok(test_server)
    }

    pub(crate) fn server_address<'a>(&'a self) -> &'a str {
        &self.server_address
    }

    /// Creates a `TestServer` running your app on the address given.
    pub(crate) fn new_with_address(
        app: IntoMakeService<Router>,
        socket_address: SocketAddr,
    ) -> Result<Self> {
        let listener = TcpListener::bind(socket_address)
            .with_context(|| "Failed to create TCPListener for TestServer")?;
        let server_address = socket_address.to_string();
        let server = Server::from_tcp(listener)
            .with_context(|| "Failed to create ::axum::Server for TestServer")?
            .serve(app);

        let server_thread = spawn(async move {
            server.await.expect("Expect server to start serving");
        });

        let test_server = Self {
            server_thread,
            server_address,
            cookies: CookieJar::new(),
        };

        Ok(test_server)
    }

    pub(crate) fn cookies<'a>(&'a self) -> &'a CookieJar {
        &self.cookies
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn add_cookies_by_header<'a, I>(
        this: &mut Arc<Mutex<Self>>,
        cookie_headers: I,
    ) -> Result<()>
    where
        I: Iterator<Item = &'a HeaderValue>,
    {
        let mut this_locked = this.lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for `add_cookies`, {:?}",
                err
            )
        })?;

        for cookie_header in cookie_headers {
            let cookie_header_str = cookie_header
                .to_str()
                .context(&"Reading cookie header for storing in the `TestServer`")
                .unwrap();

            let cookie: Cookie<'static> = Cookie::parse(cookie_header_str)?.into_owned();
            this_locked.cookies.add(cookie);
        }

        Ok(())
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn add_cookies(this: &mut Arc<Mutex<Self>>, cookies: CookieJar) -> Result<()> {
        let mut this_locked = this.lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for `add_cookies`, {:?}",
                err
            )
        })?;

        for cookie in cookies.iter() {
            this_locked.cookies.add(cookie.to_owned());
        }

        Ok(())
    }

    pub(crate) fn add_cookie(this: &mut Arc<Mutex<Self>>, cookie: Cookie) -> Result<()> {
        let mut this_locked = this.lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for `add_cookies`, {:?}",
                err
            )
        })?;

        this_locked.cookies.add(cookie.into_owned());

        Ok(())
    }

    pub(crate) fn send(this: &Arc<Mutex<Self>>, method: Method, path: &str) -> Result<TestRequest> {
        TestRequest::new(this.clone(), method, path)
    }
}

impl Drop for InnerTestServer {
    fn drop(&mut self) {
        self.server_thread.abort();
    }
}
