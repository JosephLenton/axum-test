use crate::internals::ErrorMessage;
use anyhow::Result;
use cookie::Cookie;
use cookie::CookieJar;
use http::HeaderValue;

#[derive(Debug, Clone)]
pub struct CrossCookieJar {
    is_saving_cookies: bool,
    inner: CookieJar,
}

impl CrossCookieJar {
    pub fn new(is_saving_cookies: bool) -> Self {
        Self {
            is_saving_cookies,
            inner: CookieJar::new(),
        }
    }

    pub fn is_saving(&mut self) -> bool {
        self.is_saving_cookies
    }

    pub fn enable_saving(&mut self) {
        self.is_saving_cookies = true;
    }

    pub fn disable_saving(&mut self) {
        self.is_saving_cookies = false;
    }

    pub fn clear_cookies(&mut self) {
        self.inner = CookieJar::new();
    }

    pub fn add_cookie(&mut self, cookie: Cookie) {
        self.inner.add(cookie.into_owned());
    }

    pub fn add_cookies_by_jar(&mut self, cookies: CookieJar) {
        for cookie in cookies.iter() {
            self.inner.add(cookie.to_owned());
        }
    }

    /// Adds the given cookies.
    ///
    /// They will be stored over the top of the existing cookies.
    pub fn add_cookies_by_headers<'a, I>(&mut self, cookie_headers: I) -> Result<()>
    where
        I: Iterator<Item = &'a HeaderValue>,
    {
        for cookie_header in cookie_headers {
            let cookie_header_str = cookie_header
                .to_str()
                .error_message("Reading cookie header for storing in the `TestServer`");

            let cookie = Cookie::parse(cookie_header_str)?.into_owned();
            self.inner.add(cookie);
        }

        Ok(())
    }

    #[cfg(feature = "reqwest")]
    pub fn save_cookies_by_headers<'a, I>(&mut self, cookie_headers: I) -> Result<()>
    where
        I: Iterator<Item = &'a HeaderValue>,
    {
        if !self.is_saving_cookies {
            return Ok(());
        }

        self.add_cookies_by_headers(cookie_headers)
    }

    #[cfg(feature = "reqwest")]
    pub fn to_header_value(&self) -> Option<HeaderValue> {
        let cookie_string = self
            .inner
            .iter()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");

        if cookie_string.is_empty() {
            return None;
        }

        let header_value = HeaderValue::from_str(&cookie_string).unwrap();
        Some(header_value)
    }

    pub fn to_cookie_jar(&self) -> CookieJar {
        self.inner.clone()
    }
}
