use axum::{
    body::{self, Bytes},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, Router},
};
use std::convert::Infallible;
use tower_http::trace::TraceLayer;

const MAX_FILE_SIZE: u64 = 1 * 1024 * 1024 * 1024; // 1 GB

async fn encode(file: Bytes) -> Result<impl IntoResponse, Infallible> {
    if file.len() as u64 > MAX_FILE_SIZE {
        return Ok((StatusCode::BAD_REQUEST, "File too large"));
    }
    
    // Placeholder for encoding logic here

    Ok((StatusCode::OK, "File encoded successfully"))
}

async fn decode(file: Bytes) -> Result<impl IntoResponse, Infallible> {
    if file.len() as u64 > MAX_FILE_SIZE {
        return Ok((StatusCode::BAD_REQUEST, "File too large"));
    }

    // Placeholder for decoding logic here

    Ok((StatusCode::OK, "File decoded successfully"))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/encode", post(encode))
        .route("/decode", post(decode))
        .layer(TraceLayer::new_for_http());
    
    // Run the Axum server
    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}