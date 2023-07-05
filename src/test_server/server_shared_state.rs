use ::anyhow::Context;
use ::anyhow::Result;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::hyper::http::HeaderValue;
use ::serde::Serialize;
use ::std::sync::Arc;
use ::std::sync::Mutex;

use crate::internals::QueryParamsStore;
use crate::util::with_this_mut;

#[derive(Debug)]
pub(crate) struct ServerSharedState {
    cookies: CookieJar,
    query_params: QueryParamsStore,
}

impl ServerSharedState {
    pub(crate) fn new() -> Self {
        Self {
            cookies: CookieJar::new(),
            query_params: QueryParamsStore::new(),
        }
    }

    pub(crate) fn cookies<'a>(&'a self) -> &'a CookieJar {
        &self.cookies
    }

    pub(crate) fn query_params<'a>(&'a self) -> &'a QueryParamsStore {
        &self.query_params
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
        with_this_mut(this, "add_cookies_by_header", |this| {
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
        with_this_mut(this, "clear_cookies", |this| {
            this.cookies = CookieJar::new();
        })
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn add_cookies(this: &mut Arc<Mutex<Self>>, cookies: CookieJar) -> Result<()> {
        with_this_mut(this, "add_cookies", |this| {
            for cookie in cookies.iter() {
                this.cookies.add(cookie.to_owned());
            }
        })
    }

    pub(crate) fn add_cookie(this: &mut Arc<Mutex<Self>>, cookie: Cookie) -> Result<()> {
        with_this_mut(this, "add_cookie", |this| {
            this.cookies.add(cookie.into_owned());
        })
    }

    pub(crate) fn add_query_params<V>(this: &mut Arc<Mutex<Self>>, query_params: V) -> Result<()>
    where
        V: Serialize
    {
        with_this_mut(this, "add_query_params", |this| {
            this.query_params.add(query_params)
        })?
    }
}
