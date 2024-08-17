use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use flashpoint_archive::{
    game::{Game, PartialGame},
    game_data::{GameData, PartialGameData},
};

pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Game>, AppError> {
    let archive = state.archive.read().await;
    match archive.find_game(&id).await {
        Ok(Some(game)) => Ok(Json(game)),
        Ok(None) => Err(AppError::NotFound),
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn create(
    State(state): State<AppState>,
    Json(mut game): Json<PartialGame>,
) -> Result<Json<Game>, AppError> {
    let archive = state.archive.write().await;
    match archive.create_game(&mut game).await {
        Ok(game) => Ok(Json(game)),
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Result<(), AppError> {
    let archive = state.archive.write().await;
    match archive.delete_game(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn save(
    State(state): State<AppState>,
    Json(mut game): Json<PartialGame>,
) -> Result<Json<Game>, AppError> {
    let archive = state.archive.write().await;
    match archive.save_game(&mut game).await {
        Ok(game) => Ok(Json(game)),
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn save_game_data(
    State(state): State<AppState>,
    Json(gd): Json<PartialGameData>,
) -> Result<Json<GameData>, AppError> {
    let archive = state.archive.write().await;
    match archive.save_game_data(&gd).await {
        Ok(gd) => Ok(Json(gd)),
        Err(_) => Err(AppError::NotFound),
    }
}

// pub async fn search_games(Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>, Json(query): Json<GameSearch>)
//     -> Result<Json<Vec<Game>>, AppError> {
//     let archive = db_lock.read().await;
//     match archive.search_games(&query).await {
//         Ok(games) => {
//             return Ok(Json(games));
//         },
//         Err(_) => Err(AppError::NotFound),
//     }
// }

// #[derive(Deserialize, Serialize)]
// pub struct SearchInputQuery {
//     text: String
// }

// pub async fn parse_user_search_input(Json(input): Json<SearchInputQuery>)
//     -> Json<ParsedInput> {
//     Json(parse_user_input(&input.text))
// }
