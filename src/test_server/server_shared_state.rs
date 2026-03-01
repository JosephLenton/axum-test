use crate::internals::Uri2;
use anyhow::Result;
use http::HeaderName;
use http::HeaderValue;
use serde::Serialize;
use std::error::Error as StdError;
use std::fmt::Debug;

#[derive(Debug)]
pub(crate) struct ServerSharedState {
    server_uri: Uri2,
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl ServerSharedState {
    pub(crate) fn new(server_uri: Uri2) -> Self {
        Self {
            server_uri,
            headers: Vec::new(),
        }
    }

    pub(crate) fn uri(&self) -> &Uri2 {
        &self.server_uri
    }

    pub(crate) fn headers(&self) -> &Vec<(HeaderName, HeaderValue)> {
        &self.headers
    }

    pub(crate) fn add_query_params<V>(&mut self, query_params: V) -> Result<()>
    where
        V: Serialize,
    {
        self.server_uri.add_query_params(query_params)
    }

    pub(crate) fn add_raw_query_param(&mut self, raw_value: &str) {
        self.server_uri.add_raw_query_param(raw_value);
    }

    pub(crate) fn clear_query_params(&mut self) {
        self.server_uri.clear_query_params();
    }

    pub fn add_header<N, V>(&mut self, name: N, value: V) -> Result<()>
    where
        N: TryInto<HeaderName>,
        N::Error: StdError + Send + Sync + 'static,
        V: TryInto<HeaderValue>,
        V::Error: StdError + Send + Sync + 'static,
    {
        let header_name = name.try_into()?;
        let header_value = value.try_into()?;

        self.headers.push((header_name, header_value));

        Ok(())
    }

    pub(crate) fn clear_headers(&mut self) {
        self.headers.clear()
    }
}
