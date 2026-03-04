use crate::internals::QueryParamsStore;
use anyhow::Error as AnyhowError;
use anyhow::Result;
use anyhow::anyhow;
use http::Error as UriError;
use http::Uri;
use http::uri::Authority;
use http::uri::InvalidUri;
use http::uri::Scheme;
use serde::Serialize;
use std::fmt::Debug;
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
    authority: Option<Authority>,
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
            authority: uri.authority().cloned(),
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
            authority: Some(
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

    pub fn set_scheme<S>(&mut self, scheme_raw: S) -> Result<(), InvalidUri>
    where
        S: TryInto<Scheme, Error = InvalidUri>,
    {
        let scheme = scheme_raw.try_into()?;
        self.scheme = Some(scheme);

        Ok(())
    }

    pub fn set_authority<A>(&mut self, authority_raw: A) -> Result<(), InvalidUri>
    where
        A: TryInto<Authority, Error = InvalidUri>,
    {
        let authority = authority_raw.try_into()?;
        self.authority = Some(authority);

        Ok(())
    }

    pub fn has_authority(&self) -> bool {
        self.authority.is_some()
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

    pub fn set_path_from_uri(&mut self, uri: &Uri) {
        match (uri.authority(), uri.path()) {
            (Some(authority), "") => {
                self.set_path(authority);
            }
            (None, path) => {
                self.set_path(path);
            }
            _ => {
                panic!("Cannot decipher path given, expected a relative url like '/todo'")
            }
        }
    }

    pub fn set_path<S>(&mut self, path: S)
    where
        S: ToString,
    {
        self.path = path.to_string();
    }

    pub fn add_query_from_uri(&mut self, uri: &Uri) {
        if let Some(query) = uri.query() {
            self.query.add_raw(query.to_string());
        }
    }

    /// This is a large function that allows overriding the existing URI with 'something' given by the user.
    /// What happens depends on a bunch of custom logic.
    pub fn set_uri_str(&mut self, other: &str, is_http_restricted: bool) -> Result<()> {
        let other_uri = other.parse::<Uri>()?;

        //
        // Why does this exist?
        //
        // This exists to allow `server.get("/users")` and `server.get("users")` (without a slash)
        // to go to the same place.
        //
        // It does this by saying ...
        //  - if there is a scheme, it's a full uri.
        //  - if no scheme, it must be a path
        //
        // If there is a scheme, then this is an absolute path.
        if let Some(scheme) = other_uri.scheme() {
            println!(" ... has scheme");
            if is_http_restricted {
                let has_different_scheme = other_uri
                    .scheme()
                    .is_some_and(|other_scheme| Some(other_scheme) != self.scheme.as_ref());
                let has_different_authority =
                    other_uri.authority().is_some_and(|other_authority| {
                        Some(other_authority) != self.authority.as_ref()
                    });

                if has_different_scheme || has_different_authority {
                    return Err(anyhow!(
                        "Request disallowed for path '{other}', requests are only allowed to local server. Turn off 'restrict_requests_with_http_scheme' to change this."
                    ));
                }
            } else {
                self.scheme = Some(scheme.clone());

                // We only set the host/port if the scheme is also present.
                if let Some(authority) = other_uri.authority() {
                    self.authority = Some(authority.clone());
                }
            }

            self.set_path(other_uri.path());

            // In this path we are replacing, so drop any query params on the original url.
            self.clear_query_params();
        } else {
            // Grab everything up until the query parameters, or everything after that
            let calculated_path = other.split('?').next().unwrap_or(other);

            // TODO: adding the slash should happen as late as possible, in the to_uri / into_uri methods.
            let path = if calculated_path.starts_with("/") {
                calculated_path.to_string()
            } else {
                format!("/{calculated_path}")
            };

            self.set_path(path);
        }

        if let Some(path_query) = other_uri.query() {
            self.add_raw_query_param(path_query);
        }

        Ok(())
    }

    pub fn to_uri(&self) -> Result<Uri, UriError> {
        self.clone().into_uri()
    }

    pub fn into_uri(self) -> Result<Uri, UriError> {
        let mut uri_builder = Uri::builder();

        let path_and_query = self.to_path_and_query();
        let has_scheme = self.scheme.is_some();
        let has_authority = self.authority.is_some();

        if let Some(scheme) = self.scheme {
            uri_builder = uri_builder.scheme(scheme);

            if !has_authority {
                let localhost_authority = Authority::from_static("localhost");
                uri_builder = uri_builder.authority(localhost_authority);
            }
        }

        if let Some(authority) = self.authority {
            uri_builder = uri_builder.authority(authority);

            if !has_scheme {
                uri_builder = uri_builder.scheme(Scheme::HTTP);
            }
        }

        uri_builder = uri_builder.path_and_query(path_and_query);

        uri_builder.build()
    }

    pub fn to_url(&self) -> Result<Url> {
        self.clone().into_url()
    }

    pub fn into_url(self) -> Result<Url> {
        self.to_string().parse().map_err(Into::into)
    }

    fn to_path_and_query(&self) -> String {
        if self.query.is_empty() {
            return self.path.to_string();
        }

        return format!("{}?{}", self.path, self.query);
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
        uri2.into_uri()
    }
}

impl TryFrom<&Uri2> for Url {
    type Error = AnyhowError;

    fn try_from(uri2: &Uri2) -> Result<Self, Self::Error> {
        uri2.to_url()
    }
}

impl TryFrom<Uri2> for Url {
    type Error = AnyhowError;

    fn try_from(uri2: Uri2) -> Result<Self, Self::Error> {
        uri2.into_url()
    }
}

impl Display for Uri2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let uri = self
            .to_uri()
            .expect("The Uri2 should always turn into a Uri");

        Display::fmt(&uri, f)
    }
}

#[cfg(test)]
mod test_fmt {
    use super::*;

    #[test]
    fn it_should_format_the_example_http_domain() {
        let mut uri2 = Uri2::new();
        uri2.set_scheme("http").unwrap();
        uri2.set_authority("example.com").unwrap();

        let output = uri2.to_string();
        assert_eq!(output, "http://example.com/");
    }
}

#[cfg(test)]
mod test_add_query_from_uri {
    use super::*;

    #[test]
    fn todo() {
        todo!()
    }
}

#[cfg(test)]
mod test_set_uri_str {
    use super::*;

    #[test]
    fn it_should_copy_path_to_url_returned_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "/users";
        base_url.set_uri_str(&path, true).unwrap();

        assert_eq!("http://example.com/users", base_url.to_string());
    }

    #[test]
    fn it_should_copy_all_query_params_to_store_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com?base=aaa");
        let path = "/users?path=bbb&path-flag";
        base_url.set_uri_str(&path, true).unwrap();

        assert_eq!(
            "http://example.com/users?base=aaa&path=bbb&path-flag",
            base_url.to_string()
        );
    }

    #[test]
    fn it_should_not_replace_url_when_restricted_with_different_scheme() {
        let mut base_url = Uri2::from_str("http://example.com?base=666");
        let path = "ftp://google.com:123/users.csv?limit=456";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_not_replace_url_when_restricted_with_same_scheme() {
        let mut base_url = Uri2::from_str("http://example.com?base=666");
        let path = "http://google.com:123/users.csv?limit=456";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_block_url_when_restricted_with_same_scheme() {
        let mut base_url = Uri2::from_str("http://example.com?base=666");
        let path = "http://google.com";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_block_url_when_restricted_and_same_domain_with_different_scheme() {
        let mut base_url = Uri2::from_str("http://example.com?base=666");
        let path = "ftp://example.com/users";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_copy_path_to_url_returned_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "/users";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!("http://example.com/users", base_url.to_string());
    }

    #[test]
    fn it_should_copy_all_query_params_to_store_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com?base=aaa");
        let path = "/users?path=bbb&path-flag";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!(
            "http://example.com/users?base=aaa&path=bbb&path-flag",
            base_url.to_string()
        );
    }

    #[test]
    fn it_should_copy_host_like_a_path_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "google.com";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!("http://example.com/google.com", base_url.to_string());
    }

    #[test]
    fn it_should_copy_host_like_a_path_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "google.com";
        base_url.set_uri_str(&path, true).unwrap();

        assert_eq!("http://example.com/google.com", base_url.to_string());
    }

    #[test]
    fn it_should_replace_url_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com?base=666");
        let path = "ftp://google.com:123/users.csv?limit=456";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!(
            "ftp://google.com:123/users.csv?limit=456",
            base_url.to_string()
        );
    }

    #[test]
    fn it_should_allow_different_scheme_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "ftp://example.com";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!("ftp://example.com/", base_url.to_string());
    }

    #[test]
    fn it_should_allow_different_host_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "http://google.com";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!("http://google.com/", base_url.to_string());
    }

    #[test]
    fn it_should_allow_different_port_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com:123");
        let path = "http://example.com:456";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!("http://example.com:456/", base_url.to_string());
    }

    #[test]
    fn it_should_allow_same_host_port_when_unrestricted() {
        let mut base_url = Uri2::from_str("http://example.com:123");
        let path = "http://example.com:123";
        base_url.set_uri_str(&path, false).unwrap();

        assert_eq!("http://example.com:123/", base_url.to_string());
    }

    #[test]
    fn it_should_not_allow_different_scheme_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "ftp://example.com";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_not_allow_different_host_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com");
        let path = "http://google.com";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_not_allow_different_port_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com:123");
        let path = "http://example.com:456";
        let result = base_url.set_uri_str(&path, true);

        assert!(result.is_err());
    }

    #[test]
    fn it_should_allow_same_host_port_when_restricted() {
        let mut base_url = Uri2::from_str("http://example.com:123");
        let path = "http://example.com:123";
        base_url.set_uri_str(&path, true).unwrap();

        assert_eq!("http://example.com:123/", base_url.to_string());
    }
}
