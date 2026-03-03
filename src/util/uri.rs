use http::uri::Uri;

pub fn is_absolute_uri(path_uri: &Uri) -> bool {
    path_uri.scheme_str().is_some()
}
