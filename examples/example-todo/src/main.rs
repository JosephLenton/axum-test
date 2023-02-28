//!
//! This is an example Todo Application.
//! To show how one can write some simple tests.
//!
//! It's all one file, and kept to the absolutely bare minimum.
//! Scroll to the bottom to see the tests.
//!

use ::anyhow::anyhow;
use ::anyhow::Result;
use ::axum::extract::Json;
use ::axum::extract::State;
use ::axum::http::StatusCode;
use ::axum::routing::get;
use ::axum::routing::post;
use ::axum::routing::put;
use ::axum::routing::IntoMakeService;
use ::axum::Router;
use ::axum::Server;
use ::axum_extra::extract::cookie::Cookie;
use ::axum_extra::extract::cookie::CookieJar;
use ::serde::Deserialize;
use ::serde::Serialize;
use ::serde_email::Email;
use ::std::collections::HashMap;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;
use ::std::result::Result as StdResult;

const PORT: u16 = 8080;
const USER_ID_COOKIE_NAME: &'static str = &"example-todo-user-id";

#[tokio::main]
async fn main() -> Result<()> {
    let result = {
        let app = new_app();

        // Start!
        let ip_address = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let address = SocketAddr::new(ip_address, PORT);
        Server::bind(&address).serve(app).await.unwrap();

        Ok(())
    };

    match &result {
        Err(err) => eprintln!("{}", err),
        _ => {}
    };

    result
}

// This my poor mans in memory DB.
#[derive(Clone, Debug)]
pub struct AppState {
    user_todos: HashMap<u32, Vec<String>>,
}

pub(crate) fn new_app() -> IntoMakeService<Router> {
    let state = AppState {
        user_todos: HashMap::new(),
    };

    let router: Router = Router::new()
        .route(&"/login", post(route_post_user_login))
        .route(&"/todo", get(route_get_user_todos))
        .route(&"/todo", put(route_put_user_todos))
        .with_state(state);

    router.into_make_service()
}

// Never do something like this in a real application.
// It's really bad. Like _seriously_ bad.
//
// Do stuff like this and users can login as each other.
// You are asking for trouble.
fn get_user_id_from_cookie(cookies: &CookieJar) -> Result<u32> {
    cookies
        .get(&USER_ID_COOKIE_NAME)
        .map(|c| c.value().to_string().parse::<u32>().ok())
        .flatten()
        .ok_or_else(|| anyhow!("id not found"))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    user: Email,
}

pub async fn route_post_user_login(
    State(ref mut state): State<AppState>,
    mut cookies: CookieJar,
    Json(body): Json<LoginRequest>,
) -> StdResult<CookieJar, StatusCode> {
    get_user_id_from_cookie(&mut cookies)
        .map(|user_id| {
            let user_id = state.user_todos.len() as u32;
            state.user_todos.insert(user_id, vec![]);

            let really_insecure_login_cookie =
                Cookie::new(USER_ID_COOKIE_NAME, user_id.to_string());
            cookies = cookies.add(really_insecure_login_cookie);

            cookies
        })
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub async fn route_get_user_todos(
    State(ref state): State<AppState>,
    cookies: CookieJar,
) -> StdResult<(), StatusCode> {
    let result = ();

    Ok(result)
}

pub async fn route_put_user_todos(
    State(ref state): State<AppState>,
    cookies: CookieJar,
) -> StdResult<(), StatusCode> {
    let result = ();

    Ok(result)
}

#[cfg(test)]
mod test_post_login {
    // todo
}
