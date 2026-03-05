use crate::internals::QueryParamsStore;
use anyhow::Result;
use http::HeaderName;
use http::HeaderValue;
use serde::Serialize;

#[derive(Debug)]
pub(crate) struct ServerSharedState {
    query_params: QueryParamsStore,
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl ServerSharedState {
    pub(crate) fn new() -> Self {
        Self {
            query_params: QueryParamsStore::new(),
            headers: Vec::new(),
        }
    }

    pub(crate) fn query_params(&self) -> &QueryParamsStore {
        &self.query_params
    }

    pub(crate) fn headers(&self) -> &Vec<(HeaderName, HeaderValue)> {
        &self.headers
    }

    pub(crate) fn add_query_params<V>(&mut self, query_params: V) -> Result<()>
    where
        V: Serialize,
    {
        self.query_params.add(query_params)
    }

    pub(crate) fn add_query_param<V>(&mut self, key: &str, value: V) -> Result<()>
    where
        V: Serialize,
    {
        self.query_params.add(&[(key, value)])
    }

    pub(crate) fn add_raw_query_param(&mut self, raw_value: &str) {
        self.query_params.add_raw(raw_value.to_string());
    }

    pub(crate) fn clear_query_params(&mut self) {
        self.query_params.clear();
    }

    pub(crate) fn clear_headers(&mut self) {
        self.headers.clear();
    }

    pub(crate) fn add_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.push((name, value));
    }
}
