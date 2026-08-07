mod routes;

use axum::Router;

#[tokio::main]
async fn main() {
    let app = Router::new().nest("/api", routes::router());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
