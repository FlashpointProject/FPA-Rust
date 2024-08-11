use std::sync::Arc;

use axum::{extract::Path, Extension, Json};
use flashpoint_archive::{game::{search::{parse_user_input, GameSearch, ParsedInput}, Game}, FlashpointArchive};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, time::Instant};

use crate::error::AppError;

pub async fn find_game(Path(id): Path<String>, Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>)
    -> Result<Json<Game>, AppError> {
    let archive = db_lock.read().await;
    match archive.find_game(&id).await {
        Ok(game) => {
            if let Some(game) = game {
                return Ok(Json(game));
            }
            Err(AppError::NotFound)
        },
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn search_games(Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>, Json(query): Json<GameSearch>)
    -> Result<Json<Vec<Game>>, AppError> {
    let archive = db_lock.read().await;
    match archive.search_games(&query).await {
        Ok(games) => {
            return Ok(Json(games));
        },
        Err(_) => Err(AppError::NotFound),
    }
}

#[derive(Deserialize, Serialize)]
pub struct SearchInputQuery {
    text: String
}

pub async fn parse_user_search_input(Json(input): Json<SearchInputQuery>)
    -> Json<ParsedInput> {
    Json(parse_user_input(&input.text))
}