use axum::{Router, extract::OriginalUri, extract::Request, response::IntoResponse, routing::get};

async fn handler(request: Request) -> impl IntoResponse {
    println!("{request:#?}");
    let (parts, _) = request.into_parts();
    let scheme = parts
        .uri
        .scheme_str()
        .unwrap_or(" ... unknown from uri.parts ... ");
    scheme.to_string()
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
