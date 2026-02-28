use crate::internals::QueryParamsStore;
use anyhow::Result;
use http::Error as UriError;
use http::Uri;
use http::uri::Authority;
use http::uri::Scheme;
use serde::Serialize;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use url::Url;

/// This exists as an alternative to the pains and restrictions of `url::Url`, and `http::Uri`.
///
///  - `url::Url` is great for building and manipulating a url, however _all_ urls are required to have a domain.
///  - `http::Uri` allows both absolute and relative urls, however it's building and manipulation sucks.
///
/// `Uri2` offers URIs with a better interface for Axum Test.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Uri2 {
    scheme: Option<Scheme>,
    host: Option<Authority>,
    path: String,
    query: QueryParamsStore,
}

impl Uri2 {
    #[cfg(test)]
    pub fn from_str(uri: &str) -> Self {
        use std::str::FromStr;

        let uri = Uri::from_str(uri).expect("Failed to parse uri");
        Self::from_uri(&uri)
    }

    pub fn from_uri(uri: &Uri) -> Self {
        Self {
            scheme: uri.scheme().cloned(),
            host: uri.authority().cloned(),
            path: uri.path().to_string(),
            query: QueryParamsStore::from_uri(uri),
        }
    }

    pub fn from_url(url: &Url) -> Self {
        Self {
            scheme: Some(
                url.scheme()
                    .parse()
                    .expect("The given url should have a valid scheme"),
            ),
            host: Some(
                url.authority()
                    .parse()
                    .expect("The given url should have a valid authority"),
            ),
            path: url.path().to_string(),
            query: QueryParamsStore::from_url(url),
        }
    }

    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_scheme(&mut self, scheme_str: &str) -> Result<()> {
        let scheme = scheme_str.parse()?;
        self.scheme = Some(scheme);

        Ok(())
    }

    pub fn add_query_params<V>(&mut self, query_params: V) -> Result<()>
    where
        V: Serialize,
    {
        self.query.add(query_params)
    }

    pub fn add_raw_query_param(&mut self, query_param: &str) {
        self.query.add_raw(query_param.to_string());
    }

    pub fn clear_query_params(&mut self) {
        self.query.clear();
    }

    /*
    pub fn push(&mut self, other: Self) {
        if other.scheme.is_some() {
            self.scheme = other.scheme;
        }

        if other.host.is_some() {
            self.host = other.host;
        }

        self.path = other.path;
    }
     */

    pub fn to_uri(&self) -> Result<Uri, UriError> {
        let mut uri_builder = Uri::builder();

        if let Some(scheme) = &self.scheme {
            uri_builder = uri_builder.scheme(scheme.clone());
        }

        if let Some(authority) = &self.host {
            uri_builder = uri_builder.authority(authority.clone());
        }

        let path_and_query = format!("{}?{}", self.path, self.query);
        uri_builder = uri_builder.path_and_query(path_and_query);

        uri_builder.build()
    }
}

impl TryFrom<&Uri2> for Uri {
    type Error = UriError;

    fn try_from(uri2: &Uri2) -> Result<Self, Self::Error> {
        uri2.to_uri()
    }
}

impl TryFrom<Uri2> for Uri {
    type Error = UriError;

    fn try_from(uri2: Uri2) -> Result<Self, Self::Error> {
        let mut uri_builder = Uri::builder();

        if let Some(scheme) = uri2.scheme {
            uri_builder = uri_builder.scheme(scheme);
        }

        if let Some(authority) = uri2.host {
            uri_builder = uri_builder.authority(authority);
        }

        let path_and_query = format!("{}?{}", uri2.path, uri2.query);
        uri_builder = uri_builder.path_and_query(path_and_query);

        uri_builder.build()
    }
}

impl Display for Uri2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        todo!()
    }
}
