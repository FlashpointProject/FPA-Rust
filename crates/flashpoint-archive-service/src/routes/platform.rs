use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use flashpoint_archive::{
    tag::{PartialTag, Tag},
    FlashpointArchive,
};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{error::AppError, AppState};

pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Tag>, AppError> {
    let archive = state.archive.read().await;
    if let Ok(int_id) = id.parse::<i64>() {
        // If the id can be parsed as an integer, use get_tag_by_id
        match archive.find_platform_by_id(int_id).await {
            Ok(Some(platform)) => Ok(Json(platform)),
            Ok(None) => Err(AppError::NotFound),
            Err(_) => Err(AppError::NotFound),
        }
    } else {
        // If the id is not an integer, use find_tag
        match archive.find_platform(&id).await {
            Ok(Some(platform)) => Ok(Json(platform)),
            Ok(None) => Err(AppError::NotFound),
            Err(_) => Err(AppError::NotFound),
        }
    }
}

#[derive(Deserialize)]
pub struct CreatePlatformData {
    name: String,
    id: Option<i64>,
}

pub async fn create(
    Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>,
    Json(data): Json<CreatePlatformData>,
) -> Result<Json<Tag>, AppError> {
    let archive = db_lock.write().await;
    match archive.create_platform(&data.name, data.id).await {
        Ok(platform) => Ok(Json(platform)),
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn delete(
    Path(id): Path<String>,
    Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>,
) -> Result<(), AppError> {
    let archive = db_lock.write().await;
    match archive.delete_tag(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(AppError::NotFound),
    }
}

pub async fn save(
    Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>,
    Json(mut platform): Json<PartialTag>,
) -> Result<Json<Tag>, AppError> {
    let archive = db_lock.write().await;
    match archive.save_platform(&mut platform).await {
        Ok(platform) => Ok(Json(platform)),
        Err(_) => Err(AppError::NotFound),
    }
}
