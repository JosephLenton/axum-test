use http::uri::Uri;
use url::Url;

pub fn is_absolute_uri(path_uri: &Uri) -> bool {
    path_uri.scheme_str().is_some()
}

pub fn has_different_scheme(base_url: &Url, path_uri: &Uri) -> bool {
    if let Some(scheme) = path_uri.scheme_str() {
        return scheme != base_url.scheme();
    }

    false
}

pub fn has_different_authority(base_url: &Url, path_uri: &Uri) -> bool {
    if let Some(authority) = path_uri.authority() {
        return authority.as_str() != base_url.authority();
    }

    false
}
