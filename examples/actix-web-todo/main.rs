//!
//! This is an example Todo Application using Actix Web to show some simple tests.
//!
//! ```bash
//! # To run it's tests:
//! cargo test --example=actix-web-todo --features actix-web
//! ```
//!
//! The app includes the end points for ...
//!
//!  - POST /login ... this takes an email, and returns a session cookie.
//!  - PUT /todo ... once logged in, one can store todos.
//!  - GET /todo ... once logged in, you can retrieve all todos you have stored.
//!
//! At the bottom of this file are a series of tests for these endpoints.
//!

use actix_web::App;
use actix_web::Error as ActixWebError;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::body::BoxBody;
use actix_web::cookie::Cookie;
use actix_web::dev::ServiceFactory;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::get;
use actix_web::web::post;
use actix_web::web::put;
use anyhow::Result;
use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;
use serde_email::Email;
use std::collections::HashMap;
use std::io::Result as IoResult;
use std::sync::Arc;
use std::sync::RwLock;

const PORT: u16 = 8080;
const USER_ID_COOKIE_NAME: &'static str = &"todo-user-id";

#[actix_web::main]
async fn main() -> IoResult<()> {
    HttpServer::new(move || new_app())
        .bind(format!("0.0.0.0:{PORT}"))?
        .run()
        .await
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

// Note you should never do something like this in a real application
// for session cookies. It's really bad. Like _seriously_ bad.
//
// This is done like this here to keep the code shorter. That's all.
fn get_user_id_from_request(req: &HttpRequest) -> Result<u32> {
    req.cookie(USER_ID_COOKIE_NAME)
        .and_then(|c| c.value().parse::<u32>().ok())
        .ok_or_else(|| anyhow!("id not found"))
}

pub async fn route_post_user_login(
    state: Data<SharedAppState>,
    _body: Json<LoginRequest>,
) -> HttpResponse {
    let mut lock = state.write().unwrap();
    let user_id = lock.user_todos.len() as u32;
    lock.user_todos.insert(user_id, vec![]);

    let cookie = Cookie::build(USER_ID_COOKIE_NAME, user_id.to_string()).finish();

    HttpResponse::Ok().cookie(cookie).finish()
}

pub async fn route_put_user_todos(
    req: HttpRequest,
    state: Data<SharedAppState>,
    body: Json<Todo>,
) -> HttpResponse {
    let user_id = match get_user_id_from_request(&req) {
        Ok(id) => id,
        Err(_) => return HttpResponse::Unauthorized().finish(),
    };

    let mut lock = state.write().unwrap();
    let todos = match lock.user_todos.get_mut(&user_id) {
        Some(todos) => todos,
        None => return HttpResponse::Unauthorized().finish(),
    };
    todos.push(body.into_inner());
    let num_todos = todos.len() as u32;

    HttpResponse::Ok().json(num_todos)
}

pub async fn route_get_user_todos(req: HttpRequest, state: Data<SharedAppState>) -> HttpResponse {
    let user_id = match get_user_id_from_request(&req) {
        Ok(id) => id,
        Err(_) => return HttpResponse::Unauthorized().finish(),
    };

    let lock = state.read().unwrap();
    let todos = match lock.user_todos.get(&user_id) {
        Some(todos) => todos.clone(),
        None => return HttpResponse::Unauthorized().finish(),
    };

    HttpResponse::Ok().json(todos)
}

fn new_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<BoxBody>,
        Error = ActixWebError,
        InitError = (),
    >,
> {
    let state = new_app_state();

    App::new()
        .app_data(Data::new(state))
        .route("/login", post().to(route_post_user_login))
        .route("/todo", get().to(route_get_user_todos))
        .route("/todo", put().to(route_put_user_todos))
}

fn new_app_state() -> SharedAppState {
    let state = AppState {
        user_todos: HashMap::new(),
    };
    Arc::new(RwLock::new(state))
}
