use crate::error::AppError;
use auth::is_admin_middleware;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    handler::Handler,
    http::request::Parts,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use config::Config;
use flashpoint_archive::FlashpointArchive;
use flashpoint_archive::{enable_debug, tag::Tag};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

mod auth;
mod config;
mod error;
mod routes;

async fn list_tags(State(state): State<AppState>) -> Result<Json<Vec<Tag>>, AppError> {
    let archive = state.archive.read().await;
    match archive.find_all_tags(vec![]).await {
        Ok(tags) => Ok(Json(tags)),
        Err(_) => Err(AppError::InternalServerError),
    }
}

async fn list_platforms(State(state): State<AppState>) -> Result<Json<Vec<Tag>>, AppError> {
    let archive = state.archive.read().await;
    match archive.find_all_platforms().await {
        Ok(platforms) => Ok(Json(platforms)),
        Err(_) => Err(AppError::InternalServerError),
    }
}

#[derive(Clone)]
struct AppState {
    archive: Arc<RwLock<FlashpointArchive>>,
    client: Arc<BasicClient>,
    config: Arc<Config>,
    auth_pool: Arc<RwLock<Pool<SqliteConnectionManager>>>,
}

#[async_trait]
impl<S> FromRequestParts<S> for AppState
where
    Self: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self::from_ref(state))
    }
}

#[tokio::main]
async fn main() {
    let config = config::Config::from_env_or_file();

    // Get the database name from the command-line arguments or use a default value
    println!(
        "Using Config: {}",
        serde_json::to_string_pretty(&config).expect("Failed to serialize config")
    );

    // Initialize the database
    let archive = RwLock::new(FlashpointArchive::new());
    {
        let mut db = archive.write().await;
        enable_debug();
        db.load_database(&config.metadata_database)
            .expect("Failed to load database");
        println!("Database Ready");
    }

    // Set up Oauth2
    let auth_conn_manager = SqliteConnectionManager::file(&config.auth_database);
    let auth_pool =
        RwLock::new(Pool::new(auth_conn_manager).expect("Failed to create auth conn pool"));
    {
        let db = auth_pool.write().await.get().unwrap();
        create_auth_db(&db).expect("Failed to populate auth db");
        println!("Auth Database Ready");
    }

    let client = BasicClient::new(
        ClientId::new(config.oauth_client_id.clone()),
        Some(ClientSecret::new(config.oauth_client_secret.clone())),
        AuthUrl::new(config.oauth_auth_url.clone()).expect("Failed to create authorization url"),
        Some(TokenUrl::new(config.oauth_token_url.clone()).expect("Failed to create token url")),
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(
        RedirectUrl::new(config.oauth_redirect_url.clone()).expect("Failed to create redirect url"),
    );

    let app_state = AppState {
        archive: Arc::new(archive),
        client: Arc::new(client),
        config: Arc::new(config),
        auth_pool: Arc::new(auth_pool),
    };

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // build our application with a route
    let app = Router::new()
        .route("/api/tags", get(list_tags))
        .route("/api/platforms", get(list_platforms))
        .route(
            "/api/game",
            post(routes::game::create.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        // Routes - Game
        .route("/api/game/:id", get(routes::game::find))
        .route(
            "/api/game/:id",
            post(routes::game::save.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        .route(
            "/api/game/:id/data/:dataId",
            post(
                routes::game::save_game_data.layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    is_admin_middleware,
                )),
            ),
        )
        .route(
            "/api/game/:id",
            delete(routes::game::delete.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        // Routes - Tag
        .route(
            "/api/tag",
            post(routes::tag::create.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        .route("/api/tag/:id", get(routes::tag::find))
        .route(
            "/api/tag/:id",
            post(routes::tag::save.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        .route(
            "/api/tag/:id",
            delete(routes::tag::delete.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        // Routes - Platform
        .route(
            "/api/platform",
            post(
                routes::platform::create.layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    is_admin_middleware,
                )),
            ),
        )
        .route("/api/platform/:id", get(routes::platform::find))
        .route(
            "/api/platform/:id",
            post(routes::platform::save.layer(middleware::from_fn_with_state(
                app_state.clone(),
                is_admin_middleware,
            ))),
        )
        .route(
            "/api/platform/:id",
            delete(
                routes::platform::delete.layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    is_admin_middleware,
                )),
            ),
        )
        // Routes - Auth
        .route("/api/profile", get(auth::get_profile))
        .route("/login", get(auth::start_oauth))
        .route("/oauth/callback", get(auth::handle_oauth_callback))
        .nest_service("/static", ServeDir::new("static"))
        .fallback_service(ServeFile::new("index.html"))
        // .route("/games", post(game::search_games))
        // .route("/search-parser", post(game::parse_user_search_input))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
        .into_make_service_with_connect_info::<SocketAddr>();

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn create_auth_db(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Create users table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT,
            avatar_url TEXT,
            roles TEXT
        )",
        [],
    )?;

    // Create sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            user_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            ip_addr TEXT,
            created_at DATETIME NOT NULL,
            expires_at DATETIME NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id)
        )",
        [],
    )?;

    Ok(())
}
