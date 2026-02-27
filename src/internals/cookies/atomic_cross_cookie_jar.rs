use crate::internals::CrossCookieJar;
use crate::internals::ErrorMessage;
use anyhow::Result;
use cookie::Cookie;
use cookie::CookieJar;
use http::HeaderValue;
#[cfg(feature = "reqwest")]
use reqwest::cookie::CookieStore;
use std::sync::Mutex;
#[cfg(feature = "reqwest")]
use url::Url;

#[derive(Debug)]
pub struct AtomicCrossCookieJar {
    inner: Mutex<CrossCookieJar>,
}

impl AtomicCrossCookieJar {
    pub fn new(is_saving_cookies: bool) -> Self {
        Self {
            inner: Mutex::new(CrossCookieJar::new(is_saving_cookies)),
        }
    }

    pub fn is_saving(&self) -> bool {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .is_saving()
    }

    pub fn enable_saving(&self) {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .enable_saving();
    }

    pub fn disable_saving(&self) {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .disable_saving();
    }

    pub fn clear_cookies(&self) {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .clear_cookies();
    }

    pub fn add_cookie(&self, cookie: Cookie) {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .add_cookie(cookie);
    }

    pub fn add_cookies_by_jar(&self, cookies: CookieJar) {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .add_cookies_by_jar(cookies);
    }

    pub fn add_cookies_by_headers<'a, I>(&self, cookie_headers: I) -> Result<()>
    where
        I: Iterator<Item = &'a HeaderValue>,
    {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .add_cookies_by_headers(cookie_headers)
    }

    #[cfg(feature = "reqwest")]
    pub fn save_cookies_by_headers<'a, I>(&self, cookie_headers: I) -> Result<()>
    where
        I: Iterator<Item = &'a HeaderValue>,
    {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .save_cookies_by_headers(cookie_headers)
    }

    #[cfg(feature = "reqwest")]
    pub fn to_header_value(&self) -> Option<HeaderValue> {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .to_header_value()
    }

    pub fn to_cookie_jar(&self) -> CookieJar {
        self.inner
            .lock()
            .error_message("Failed to lock CookieJar")
            .to_cookie_jar()
    }
}

#[cfg(feature = "reqwest")]
impl CookieStore for AtomicCrossCookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, _url: &Url) {
        self.save_cookies_by_headers(cookie_headers)
            .error_message("Failed to save cookies from headers");
    }

    fn cookies(&self, _url: &Url) -> Option<HeaderValue> {
        self.to_header_value()
    }
}
