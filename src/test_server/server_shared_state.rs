use crate::internals::QueryParamsStore;
use crate::internals::with_this_mut;
use anyhow::Result;
use http::HeaderName;
use http::HeaderValue;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;

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
}
