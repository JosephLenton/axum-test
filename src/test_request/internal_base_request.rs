use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::cookie::time::OffsetDateTime;
use ::cookie::Cookie;
use ::cookie::CookieJar;
use ::http::header;
use ::http::request::Builder;
use ::http::HeaderName;
use ::http::HeaderValue;
use ::http::Method;
use ::http::Request;
use ::serde::Serialize;
use ::url::Url;

use crate::internals::ExpectedState;
use crate::internals::QueryParamsStore;
use crate::internals::RequestPathFormatter;
use crate::TestRequestConfig;

/// The 'base' request is used for basic request handling which is
/// needed across multiple request types.
///
/// The tl;dr is it allows us to share code across `TestRequest`,
/// and a future websockets request.
#[derive(Debug, Clone)]
pub struct InternalBaseRequest {
    is_saving_cookies: bool,
    expected_state: ExpectedState,
    content_type: Option<String>,
    full_request_url: Url,
    method: Method,
    cookies: CookieJar,
    query_params: QueryParamsStore,
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl InternalBaseRequest {
    pub fn new(config: TestRequestConfig) -> Self {
        Self {
            is_saving_cookies: config.is_saving_cookies,
            expected_state: config.expected_state,
            content_type: config.content_type,
            full_request_url: config.full_request_url,
            method: config.method,
            cookies: config.cookies,
            query_params: config.query_params,
            headers: config.headers,
        }
    }

    pub fn start_building_request(self) -> Result<(Url, Builder)> {
        let debug_request_format = self.debug_request_format().to_string();
        let url = build_url_query_params(self.full_request_url, &self.query_params);
        let mut request_builder = Request::builder().uri(url.as_str()).method(self.method);

        // Add all the headers we have.
        if let Some(content_type) = self.content_type {
            let (header_key, header_value) =
                build_content_type_header(&content_type, &debug_request_format)?;
            request_builder = request_builder.header(header_key, header_value);
        }

        // Add all the non-expired cookies as headers
        let now = OffsetDateTime::now_utc();
        for cookie in self.cookies.iter() {
            let expired = cookie
                .expires_datetime()
                .map(|expires| expires <= now)
                .unwrap_or(false);

            if !expired {
                let cookie_raw = cookie.to_string();
                let header_value = HeaderValue::from_str(&cookie_raw)?;
                request_builder = request_builder.header(header::COOKIE, header_value);
            }
        }

        // Put headers into the request
        for (header_name, header_value) in self.headers {
            request_builder = request_builder.header(header_name, header_value);
        }

        Ok((url, request_builder))
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn set_content_type(&mut self, content_type: String) {
        self.content_type = Some(content_type);
    }

    pub fn add_cookie<'c>(&mut self, cookie: Cookie<'c>) {
        self.cookies.add(cookie.into_owned());
    }

    pub fn add_cookies(&mut self, cookies: CookieJar) {
        for cookie in cookies.iter() {
            self.cookies.add(cookie.clone());
        }
    }

    pub fn clear_cookies(&mut self) {
        self.cookies = CookieJar::new();
    }

    pub fn is_saving_cookies(&self) -> bool {
        self.is_saving_cookies
    }

    pub fn do_save_cookies(&mut self) {
        self.is_saving_cookies = true;
    }

    pub fn do_not_save_cookies(&mut self) {
        self.is_saving_cookies = false;
    }

    pub fn add_query_params<V>(&mut self, query_params: V) -> Result<()>
    where
        V: Serialize,
    {
        self.query_params.add(query_params).with_context(|| {
            format!(
                "It should serialize query parameters, for request {}",
                self.debug_request_format()
            )
        })?;

        Ok(())
    }

    pub fn add_raw_query_param(&mut self, query_param: &str) {
        self.query_params.add_raw(query_param.to_string());
    }

    pub fn clear_query_params(&mut self) {
        self.query_params.clear();
    }

    pub fn add_header<'c>(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.push((name, value));
    }

    pub fn clear_headers(&mut self) {
        self.headers = vec![];
    }

    pub fn set_scheme(&mut self, scheme: &str) -> Result<()> {
        self.full_request_url
            .set_scheme(scheme)
            .map_err(|_| anyhow!("Scheme '{scheme}' cannot be set to request"))?;

        Ok(())
    }

    pub fn expected_state(&self) -> ExpectedState {
        self.expected_state
    }

    pub fn expect_success(&mut self) {
        self.expected_state = ExpectedState::Success;
    }

    pub fn expect_failure(&mut self) {
        self.expected_state = ExpectedState::Failure;
    }

    pub fn debug_request_format<'a>(&'a self) -> RequestPathFormatter<'a> {
        RequestPathFormatter::new(
            &self.method,
            &self.full_request_url.as_str(),
            Some(&self.query_params),
        )
    }
}

fn build_url_query_params(mut url: Url, query_params: &QueryParamsStore) -> Url {
    // Add all the query params we have
    if query_params.has_content() {
        url.set_query(Some(&query_params.to_string()));
    }

    url
}

fn build_content_type_header(
    content_type: &str,
    debug_request_format: &str,
) -> Result<(HeaderName, HeaderValue)> {
    let header_value = HeaderValue::from_str(content_type).with_context(|| {
        format!(
            "Failed to store header content type '{content_type}', for request {debug_request_format}"
        )
    })?;

    Ok((header::CONTENT_TYPE, header_value))
}
