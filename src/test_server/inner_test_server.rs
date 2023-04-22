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
use ::std::net::TcpListener;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::tokio::spawn;
use ::tokio::task::JoinHandle;

use crate::TestRequest;
use crate::TestRequestConfig;
use crate::TestServerConfig;

/// The `InnerTestServer` is the real server that runs.
#[derive(Debug)]
pub(crate) struct InnerTestServer {
    server_thread: JoinHandle<()>,
    server_address: String,
    cookies: CookieJar,
    save_cookies: bool,
    default_content_type: Option<String>,
}

impl InnerTestServer {
    /// Creates a `TestServer` running your app on the address given.
    pub(crate) fn new(app: IntoMakeService<Router>, config: TestServerConfig) -> Result<Self> {
        let socket_address = config.build_socket_address()?;
        let listener = TcpListener::bind(socket_address)
            .with_context(|| "Failed to create TCPListener for TestServer")?;
        let server = Server::from_tcp(listener)
            .with_context(|| "Failed to create ::axum::Server for TestServer")?
            .serve(app);

        let server_thread = spawn(async move {
            server.await.expect("Expect server to start serving");
        });

        let test_server = Self {
            server_thread,
            server_address: socket_address.to_string(),
            cookies: CookieJar::new(),
            save_cookies: config.save_cookies,
            default_content_type: config.default_content_type,
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
        InnerTestServer::with_this_mut(this, "add_cookies_by_header", |this| {
            for cookie_header in cookie_headers {
                let cookie_header_str = cookie_header
                    .to_str()
                    .context(&"Reading cookie header for storing in the `TestServer`")
                    .unwrap();

                let cookie: Cookie<'static> = Cookie::parse(cookie_header_str)?.into_owned();
                this.cookies.add(cookie);
            }

            Ok(()) as Result<()>
        })?
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn clear_cookies(this: &mut Arc<Mutex<Self>>) -> Result<()> {
        InnerTestServer::with_this_mut(this, "clear_cookies", |this| {
            this.cookies = CookieJar::new();
        })
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn add_cookies(this: &mut Arc<Mutex<Self>>, cookies: CookieJar) -> Result<()> {
        InnerTestServer::with_this_mut(this, "add_cookies", |this| {
            for cookie in cookies.iter() {
                this.cookies.add(cookie.to_owned());
            }
        })
    }

    pub(crate) fn add_cookie(this: &mut Arc<Mutex<Self>>, cookie: Cookie) -> Result<()> {
        InnerTestServer::with_this_mut(this, "add_cookie", |this| {
            this.cookies.add(cookie.into_owned());
        })
    }

    pub(crate) fn test_request_config(
        this: &Arc<Mutex<Self>>,
        method: Method,
        path: &str,
    ) -> Result<TestRequestConfig> {
        InnerTestServer::with_this(this, "test_request_config", |this| {
            let full_request_path = build_request_path(&this.server_address, path);

            TestRequestConfig {
                is_saving_cookies: this.save_cookies,
                content_type: this.default_content_type.clone(),
                full_request_path,
                method,
                path: path.to_string(),
            }
        })
    }

    pub(crate) fn send(this: &Arc<Mutex<Self>>, method: Method, path: &str) -> Result<TestRequest> {
        let config = InnerTestServer::test_request_config(this, method, path)?;

        TestRequest::new(this.clone(), config)
    }

    pub(crate) fn with_this<'a, F, R>(
        this: &'a Arc<Mutex<Self>>,
        name: &str,
        some_action: F,
    ) -> Result<R>
    where
        F: FnOnce(&mut Self) -> R + 'a,
    {
        let mut this_locked = this.lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for `{}`, {:?}",
                name,
                err,
            )
        })?;

        let result = some_action(&mut this_locked);

        Ok(result)
    }

    pub(crate) fn with_this_mut<F, R>(
        this: &mut Arc<Mutex<Self>>,
        name: &str,
        some_action: F,
    ) -> Result<R>
    where
        F: FnOnce(&mut Self) -> R,
    {
        let mut this_locked = this.lock().map_err(|err| {
            anyhow!(
                "Failed to lock InternalTestServer for `{}`, {:?}",
                name,
                err,
            )
        })?;

        let result = some_action(&mut this_locked);

        Ok(result)
    }
}

impl Drop for InnerTestServer {
    fn drop(&mut self) {
        self.server_thread.abort();
    }
}

fn build_request_path(root_path: &str, sub_path: &str) -> String {
    if sub_path == "" {
        return format!("http://{}", root_path.to_string());
    }

    if sub_path.starts_with("/") {
        return format!("http://{}{}", root_path, sub_path);
    }

    format!("http://{}/{}", root_path, sub_path)
}
