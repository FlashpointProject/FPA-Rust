mod error;

use std::env;
use std::sync::Arc;
use flashpoint_archive::{enable_debug, tag::Tag};
use tokio::sync::RwLock;
use axum::{response::Html, routing::{get, post}, Extension, Json, Router};
use flashpoint_archive::FlashpointArchive;
use crate::error::AppError;

mod game;

async fn list_tags(Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>)
    -> Result<Json<Vec<Tag>>, AppError> {
    let archive = db_lock.read().await;
    match archive.find_all_tags(vec![]).await {
        Ok(tags) => Ok(Json(tags)),
        Err(_) => Err(AppError::InternalServerError)
    }
}

async fn list_platforms(Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>)
    -> Result<Json<Vec<Tag>>, AppError> {
    let archive = db_lock.read().await;
    match archive.find_all_platforms().await {
        Ok(platforms) => Ok(Json(platforms)),
        Err(_) => Err(AppError::InternalServerError)
    }
}

#[tokio::main]
async fn main() {
    // Get the database name from the command-line arguments or use a default value
    let args: Vec<String> = env::args().collect();
    let database_name = args.get(1).map_or("flashpoint.sqlite", |s| s.as_str());
    println!("Using Database: {}", database_name);

    // Initialize the database
    let database = Arc::new(RwLock::new(FlashpointArchive::new()));
    let init_copy = database.clone();
    {
        let mut db = init_copy.write().await;
        enable_debug();
        db.load_database(database_name).expect("Failed to load database");
        println!("Database Ready");
    }

    // build our application with a route
    let app = Router::new().route("/", get(handler))
        .route("/tags", get(list_tags))
        .route("/platforms", get(list_platforms))
        .route("/game/:id", get(game::find_game))
        .route("/games", post(game::search_games))
        .route("/search-parser", post(game::parse_user_search_input))
        .layer(Extension(database.clone()));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Flashpoint Database API</h1>")
}