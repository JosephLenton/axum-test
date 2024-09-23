use anyhow::Context;
use anyhow::Result;
use cookie::Cookie;
use cookie::CookieJar;
use http::HeaderName;
use http::HeaderValue;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;

use crate::internals::with_this_mut;
use crate::internals::QueryParamsStore;

#[derive(Debug)]
pub(crate) struct ServerSharedState {
    scheme: Option<String>,
    cookies: CookieJar,
    query_params: QueryParamsStore,
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl ServerSharedState {
    pub(crate) fn new() -> Self {
        Self {
            scheme: None,
            cookies: CookieJar::new(),
            query_params: QueryParamsStore::new(),
            headers: Vec::new(),
        }
    }

    pub(crate) fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    pub(crate) fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    pub(crate) fn query_params(&self) -> &QueryParamsStore {
        &self.query_params
    }

    pub(crate) fn headers(&self) -> &Vec<(HeaderName, HeaderValue)> {
        &self.headers
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn add_cookies_by_header<'a, I>(
        this: &Arc<Mutex<Self>>,
        cookie_headers: I,
    ) -> Result<()>
    where
        I: Iterator<Item = &'a HeaderValue>,
    {
        with_this_mut(this, "add_cookies_by_header", |this| {
            for cookie_header in cookie_headers {
                let cookie_header_str = cookie_header
                    .to_str()
                    .context("Reading cookie header for storing in the `TestServer`")
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
    pub(crate) fn clear_cookies(this: &Arc<Mutex<Self>>) -> Result<()> {
        with_this_mut(this, "clear_cookies", |this| {
            this.cookies = CookieJar::new();
        })
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub(crate) fn add_cookies(this: &Arc<Mutex<Self>>, cookies: CookieJar) -> Result<()> {
        with_this_mut(this, "add_cookies", |this| {
            for cookie in cookies.iter() {
                this.cookies.add(cookie.to_owned());
            }
        })
    }

    pub(crate) fn add_cookie(this: &Arc<Mutex<Self>>, cookie: Cookie) -> Result<()> {
        with_this_mut(this, "add_cookie", |this| {
            this.cookies.add(cookie.into_owned());
        })
    }

    pub(crate) fn add_query_params<V>(this: &Arc<Mutex<Self>>, query_params: V) -> Result<()>
    where
        V: Serialize,
    {
        with_this_mut(this, "add_query_params", |this| {
            this.query_params.add(query_params)
        })?
    }

    pub(crate) fn add_query_param<V>(this: &Arc<Mutex<Self>>, key: &str, value: V) -> Result<()>
    where
        V: Serialize,
    {
        with_this_mut(this, "add_query_param", |this| {
            this.query_params.add(&[(key, value)])
        })?
    }

    pub(crate) fn add_raw_query_param(this: &Arc<Mutex<Self>>, raw_value: &str) -> Result<()> {
        with_this_mut(this, "add_raw_query_param", |this| {
            this.query_params.add_raw(raw_value.to_string())
        })
    }

    pub(crate) fn clear_query_params(this: &Arc<Mutex<Self>>) -> Result<()> {
        with_this_mut(this, "clear_query_params", |this| this.query_params.clear())
    }

    pub(crate) fn clear_headers(this: &Arc<Mutex<Self>>) -> Result<()> {
        with_this_mut(this, "clear_headers", |this| this.headers.clear())
    }

    pub(crate) fn add_header(
        this: &Arc<Mutex<Self>>,
        name: HeaderName,
        value: HeaderValue,
    ) -> Result<()> {
        with_this_mut(this, "add_header", |this| this.headers.push((name, value)))
    }

    pub(crate) fn set_scheme(this: &Arc<Mutex<Self>>, scheme: String) -> Result<()> {
        with_this_mut(this, "set_scheme", |this| this.scheme = Some(scheme))
    }

    pub(crate) fn set_scheme_unlocked(&mut self, scheme: String) {
        self.scheme = Some(scheme);
    }
}
