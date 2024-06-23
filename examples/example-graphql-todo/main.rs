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

use ::anyhow::Result;
use ::async_graphql::Context;
use ::async_graphql::EmptySubscription;
use ::async_graphql::Object;
use ::async_graphql::Schema;
use ::async_graphql::ID;
use ::async_graphql_axum::GraphQL;
use ::axum::routing::post_service;
use ::axum::serve::serve;
use ::axum::Router;
use ::serde::Deserialize;
use ::serde::Serialize;
use ::std::collections::HashMap;
use ::std::net::IpAddr;
use ::std::net::Ipv4Addr;
use ::std::net::SocketAddr;
use ::std::sync::Arc;
use ::tokio::net::TcpListener;
use ::tokio::sync::Mutex;

#[cfg(test)]
use ::axum_test::TestServer;
#[cfg(test)]
use ::axum_test::TestServerConfig;

const PORT: u16 = 8080;

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

type SharedAppState = Arc<Mutex<AppState>>;

#[derive(Debug, Default)]
pub struct AppState {
    user_todos: HashMap<u32, Vec<Todo>>,
    todo_id_counter: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Todo {
    id: ID,
    name: String,
    content: String,
}

#[Object]
impl Todo {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn content(&self) -> &str {
        &self.content
    }
}

/// Build my application, with all it's endpoints.
pub(crate) fn new_app() -> Router {
    let schema = Schema::build(AppQueryRoot, AppMutationRoot, EmptySubscription)
        .data(SharedAppState::default())
        .finish();

    Router::new().route("/", post_service(GraphQL::new(schema)))
}

/// Build a common `axum_test::TestServer` for use in our tests.
///
/// This test server will run the todo application,
/// and have any default settings I like.
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

pub struct AppQueryRoot;

#[Object]
impl AppQueryRoot {
    async fn todos(&self, ctx: &Context<'_>, user_id: u32) -> Vec<Todo> {
        let state = ctx.data_unchecked::<SharedAppState>().lock().await;
        let all_user_todos = state
            .user_todos
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| vec![]);

        all_user_todos
    }
}

pub struct AppMutationRoot;

#[Object]
impl AppMutationRoot {
    async fn create_todo(
        &self,
        ctx: &Context<'_>,
        user_id: u32,
        name: String,
        content: String,
    ) -> ID {
        let mut state = ctx.data_unchecked::<SharedAppState>().lock().await;

        // Increment ID for the next Todo
        state.todo_id_counter += 1;
        let todo_id: ID = state.todo_id_counter.into();

        // Add the new Todo
        let user_todos = state.user_todos.entry(user_id).or_default();
        user_todos.push(Todo {
            id: todo_id.clone(),
            name,
            content,
        });

        todo_id
    }

    async fn delete_todo(&self, ctx: &Context<'_>, user_id: u32, id: ID) -> Result<bool> {
        let mut state = ctx.data_unchecked::<SharedAppState>().lock().await;

        if let Some(user_todos) = state.user_todos.get_mut(&user_id) {
            let pre_size = user_todos.len();
            user_todos.retain(|todo| todo.id != id);

            let is_deleted = pre_size != user_todos.len();
            return Ok(is_deleted);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod test_todos {
    use super::*;

    use ::serde_json::json;

    #[tokio::test]
    async fn it_should_return_none_by_default() {
        let server = new_test_app();

        let response = server.post(&"/").graphql(&unimplemented!("todo")).await;

        assert_ne!(session_cookie.value(), "");
    }
}
