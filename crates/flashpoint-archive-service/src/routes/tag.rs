use std::sync::Arc;

use axum::{extract::Path, Extension, Json};
use flashpoint_archive::{
    tag::{PartialTag, Tag},
    FlashpointArchive,
};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::error::AppError;

pub async fn find(
    Path(id): Path<String>,
    Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>,
) -> Result<Json<Tag>, AppError> {
    let archive = db_lock.read().await;
    if let Ok(int_id) = id.parse::<i64>() {
        // If the id can be parsed as an integer, use get_tag_by_id
        match archive.find_tag_by_id(int_id).await {
            Ok(Some(tag)) => Ok(Json(tag)),
            Ok(None) => Err(AppError::NotFound),
            Err(_) => Err(AppError::NotFound),
        }
    } else {
        // If the id is not an integer, use find_tag
        match archive.find_tag(&id).await {
            Ok(Some(tag)) => Ok(Json(tag)),
            Ok(None) => Err(AppError::NotFound),
            Err(_) => Err(AppError::NotFound),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateTagData {
    name: String,
    category: Option<String>,
    id: Option<i64>,
}

pub async fn create(
    Extension(db_lock): Extension<Arc<RwLock<FlashpointArchive>>>,
    Json(data): Json<CreateTagData>,
) -> Result<Json<Tag>, AppError> {
    let archive = db_lock.write().await;
    match archive.create_tag(&data.name, data.category, data.id).await {
        Ok(tag) => Ok(Json(tag)),
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
    Json(mut tag): Json<PartialTag>,
) -> Result<Json<Tag>, AppError> {
    let archive = db_lock.write().await;
    match archive.save_tag(&mut tag).await {
        Ok(tag) => Ok(Json(tag)),
        Err(_) => Err(AppError::NotFound),
    }
}
