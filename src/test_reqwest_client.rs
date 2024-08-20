use reqwest::Client;
use reqwest::IntoUrl;
use reqwest::RequestBuilder;
use std::net::SocketAddr;

pub struct TestReqwestClient {
    client: Client,
    server_address: SocketAddr,
}

impl TestReqwestClient {
    /// Convenience method to make a `GET` request to a URL.
    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client.get(url)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client.post(url)
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client.put(url)
    }

    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client.patch(url)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client.delete(url)
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client.head(url)
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        self.client.request(method, url)
    }

    pub fn execute(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<Response, crate::Error>> {
        self.client.execute(request)
    }
}
