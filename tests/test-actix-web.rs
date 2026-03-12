#![cfg(feature = "actix-web")]

use actix_web::App;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::body::BoxBody;
use actix_web::cookie::Cookie;
use actix_web::dev::ServiceFactory;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::web;
use axum_test::TestServer;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ExampleResponse {
    name: String,
    age: u32,
}

async fn route_get_json() -> HttpResponse {
    HttpResponse::Ok().json(ExampleResponse {
        name: "Joe".to_string(),
        age: 20,
    })
}

async fn route_get_header() -> HttpResponse {
    HttpResponse::Ok()
        .append_header(("x-custom-header", "my-value"))
        .finish()
}

async fn route_set_cookie() -> HttpResponse {
    HttpResponse::Ok()
        .cookie(Cookie::new("my-session", "abc123"))
        .finish()
}

async fn route_check_cookie(req: HttpRequest) -> HttpResponse {
    if req.cookie("my-session").is_some() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

async fn route_get_not_found() -> HttpResponse {
    HttpResponse::NotFound().finish()
}

fn new_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .route("/json", web::get().to(route_get_json))
        .route("/header", web::get().to(route_get_header))
        .route("/set-cookie", web::get().to(route_set_cookie))
        .route("/check-cookie", web::get().to(route_check_cookie))
        .route("/not-found", web::get().to(route_get_not_found))
}

#[cfg(feature = "yaml")]
async fn route_get_yaml() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/yaml")
        .body("name: Joe\nage: 20\n")
}

#[cfg(feature = "yaml")]
fn new_app_yaml() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().route("/yaml", web::get().to(route_get_yaml))
}

mod test_assert_json_with_mock_transport {
    use super::*;

    #[tokio::test]
    async fn it_should_assert_json_response() {
        let server = TestServer::builder().mock_transport().build(new_app);

        server.get("/json").await.assert_json(&json!({
            "name": "Joe",
            "age": 20,
        }));
    }
}

#[cfg(feature = "yaml")]
#[tokio::test]
async fn it_should_assert_yaml_response() {
    let server = TestServer::builder().mock_transport().build(new_app_yaml);

    server.get("/yaml").await.assert_yaml(&ExampleResponse {
        name: "Joe".to_string(),
        age: 20,
    });
}

#[tokio::test]
async fn it_should_assert_json_response() {
    let server = TestServer::builder().http_transport().build(new_app);

    server.get("/json").await.assert_json(&json!({
        "name": "Joe",
        "age": 20,
    }));
}

#[tokio::test]
async fn it_should_send_saved_cookies_in_subsequent_requests() {
    let server = TestServer::builder()
        .mock_transport()
        .save_cookies()
        .build(new_app);

    server.get("/set-cookie").await.assert_status_ok();
    server.get("/check-cookie").await.assert_status_ok();
}

#[tokio::test]
async fn it_should_assert_header_in_response_with_mock_transport() {
    let server = TestServer::builder().mock_transport().build(new_app);

    server
        .get("/header")
        .await
        .assert_header("x-custom-header", "my-value");
}

#[tokio::test]
async fn it_should_assert_header_in_response_with_http_transport() {
    let server = TestServer::builder().http_transport().build(new_app);

    server
        .get("/header")
        .await
        .assert_header("x-custom-header", "my-value");
}

#[tokio::test]
async fn it_should_assert_status_not_found() {
    let server = TestServer::builder().http_transport().build(new_app);

    server.get("/not-found").await.assert_status_not_found();
}
