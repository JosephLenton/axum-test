//!
//! This is an example Todo Application to show some simple tests.
//!
//! The app includes the end points for ...
//!
//!  - POST /login ... this takes an email, and returns a session cookie.
//!  - PUT /todo ... once logged in, one can store todos.
//!  - GET /todo ... once logged in, you can retrieve all todos you have stored.
//!
//! At the bottom of this file are a series of tests for these endpoints.
//!

use ::anyhow::anyhow;
use ::anyhow::Result;
use ::axum::extract::Json;
use ::axum::extract::State;
use ::axum::routing::get;
use ::axum::routing::post;
use ::axum::routing::put;
use ::axum::serve::serve;
use ::axum::Router;
use ::axum_extra::extract::cookie::Cookie;
use ::axum_extra::extract::cookie::CookieJar;
use ::http::StatusCode;
use ::serde::Deserialize;
use ::serde::Serialize;
use ::serde_email::Email;
use ::std::collections::HashMap;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;
use ::std::result::Result as StdResult;
use ::std::sync::Arc;
use ::std::sync::RwLock;
use ::tokio::net::TcpListener;

#[cfg(test)]
use ::axum_test::TestServer;
#[cfg(test)]
use ::axum_test::TestServerConfig;

const PORT: u16 = 8080;
const USER_ID_COOKIE_NAME: &'static str = &"example-todo-user-id";

#[tokio::main]
async fn main() {
    let result: Result<()> = {
        let app = new_app();

        // Start!
        let ip_address = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let address = SocketAddr::new(ip_address, PORT);
        let listener = TcpListener::bind(address).await.unwrap();
        serve(listener, app.into_make_service()).await.unwrap();

        Ok(())
    };

    match &result {
        Err(err) => eprintln!("{}", err),
        _ => {}
    };
}

type SharedAppState = Arc<RwLock<AppState>>;

// This my poor mans in memory DB.
#[derive(Debug)]
pub struct AppState {
    user_todos: HashMap<u32, Vec<Todo>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Todo {
    name: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    user: Email,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllTodos {
    todos: Vec<Todo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NumTodos {
    num: u32,
}

// Note you should never do something like this in a real application
// for session cookies. It's really bad. Like _seriously_ bad.
//
// This is done like this here to keep the code shorter. That's all.
fn get_user_id_from_cookie(cookies: &CookieJar) -> Result<u32> {
    cookies
        .get(&USER_ID_COOKIE_NAME)
        .map(|c| c.value().to_string().parse::<u32>().ok())
        .flatten()
        .ok_or_else(|| anyhow!("id not found"))
}

pub async fn route_post_user_login(
    State(ref mut state): State<SharedAppState>,
    mut cookies: CookieJar,
    Json(_body): Json<LoginRequest>,
) -> CookieJar {
    let mut lock = state.write().unwrap();
    let user_todos = &mut lock.user_todos;
    let user_id = user_todos.len() as u32;
    user_todos.insert(user_id, vec![]);

    let really_insecure_login_cookie = Cookie::new(USER_ID_COOKIE_NAME, user_id.to_string());
    cookies = cookies.add(really_insecure_login_cookie);

    cookies
}

pub async fn route_put_user_todos(
    State(ref mut state): State<SharedAppState>,
    mut cookies: CookieJar,
    Json(todo): Json<Todo>,
) -> StdResult<Json<u32>, StatusCode> {
    get_user_id_from_cookie(&mut cookies)
        .map(|user_id| {
            let mut lock = state.write().unwrap();
            let maybe_todos = lock.user_todos.get_mut(&user_id);

            let todos = maybe_todos.unwrap();
            todos.push(todo);
            let num_todos = todos.len() as u32;

            Json(num_todos)
        })
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub async fn route_get_user_todos(
    State(ref state): State<SharedAppState>,
    mut cookies: CookieJar,
) -> StdResult<Json<Vec<Todo>>, StatusCode> {
    get_user_id_from_cookie(&mut cookies)
        .map(|user_id| {
            let lock = state.read().unwrap();
            let todos = lock.user_todos[&user_id].clone();

            Json(todos)
        })
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub(crate) fn new_app() -> Router {
    let state = AppState {
        user_todos: HashMap::new(),
    };
    let shared_state = Arc::new(RwLock::new(state));

    Router::new()
        .route(&"/login", post(route_post_user_login))
        .route(&"/todo", get(route_get_user_todos))
        .route(&"/todo", put(route_put_user_todos))
        .with_state(shared_state)
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    let app = new_app();
    let config = TestServerConfig::builder()
        // Preserve cookies across requests
        // for the session cookie to work.
        .save_cookies()
        .expect_success_by_default()
        .mock_transport()
        .build();

    TestServer::new_with_config(app, config).unwrap()
}

#[cfg(test)]
mod test_post_login {
    use super::*;

    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_create_session_on_login() {
        let server = new_test_app();

        let response = server
            .post(&"/login")
            .json(&json!({
                "user": "my-login@example.com",
            }))
            .await;

        let session_cookie = response.cookie(&USER_ID_COOKIE_NAME);
        assert_ne!(session_cookie.value(), "");
    }

    #[tokio::test]
    async fn it_should_not_login_using_non_email() {
        let server = new_test_app();

        let response = server
            .post(&"/login")
            .json(&json!({
                "user": "blah blah blah",
            }))
            .expect_failure()
            .await;

        // There should not be a session created.
        let cookie = response.maybe_cookie(&USER_ID_COOKIE_NAME);
        assert!(cookie.is_none());
    }
}

#[cfg(test)]
mod test_route_put_user_todos {
    use super::*;

    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_not_store_todos_without_login() {
        let server = new_test_app();

        let response = server
            .put(&"/todo")
            .json(&json!({
                "name": "shopping",
                "content": "buy eggs",
            }))
            .expect_failure()
            .await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn it_should_return_number_of_todos_as_more_are_pushed() {
        let server = new_test_app();

        server
            .post(&"/login")
            .json(&json!({
                "user": "my-login@example.com",
            }))
            .await;

        let num_todos = server
            .put(&"/todo")
            .json(&json!({
                "name": "shopping",
                "content": "buy eggs",
            }))
            .await
            .json::<u32>();
        assert_eq!(num_todos, 1);

        let num_todos = server
            .put(&"/todo")
            .json(&json!({
                "name": "afternoon",
                "content": "buy shoes",
            }))
            .await
            .json::<u32>();
        assert_eq!(num_todos, 2);
    }
}

#[cfg(test)]
mod test_route_get_user_todos {
    use super::*;

    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_not_return_todos_if_logged_out() {
        let server = new_test_app();

        let response = server
            .put(&"/todo")
            .json(&json!({
                "name": "shopping",
                "content": "buy eggs",
            }))
            .expect_failure()
            .await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn it_should_return_all_todos_when_logged_in() {
        let server = new_test_app();

        server
            .post(&"/login")
            .json(&json!({
                "user": "my-login@example.com",
            }))
            .await;

        // Push two todos.
        server
            .put(&"/todo")
            .json(&json!({
                "name": "shopping",
                "content": "buy eggs",
            }))
            .await;
        server
            .put(&"/todo")
            .json(&json!({
                "name": "afternoon",
                "content": "buy shoes",
            }))
            .await;

        // Get all todos out from the server.
        let todos = server.get(&"/todo").await.json::<Vec<Todo>>();

        let expected_todos: Vec<Todo> = vec![
            Todo {
                name: "shopping".to_string(),
                content: "buy eggs".to_string(),
            },
            Todo {
                name: "afternoon".to_string(),
                content: "buy shoes".to_string(),
            },
        ];
        assert_eq!(todos, expected_todos)
    }
}
