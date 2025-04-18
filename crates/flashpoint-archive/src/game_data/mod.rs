use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameData {
    pub id: i64,
    pub game_id: String,
    pub title: String,
    pub date_added: String,
    pub sha256: String,
    pub crc32: i32,
    pub present_on_disk: bool,
    pub path: Option<String>,
    pub size: i64,
    pub parameters: Option<String>,
    pub application_path: String,
    pub launch_command: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartialGameData {
    pub id: Option<i64>,
    pub game_id: String,
    pub title: Option<String>,
    pub date_added: Option<String>,
    pub sha256: Option<String>,
    pub crc32: Option<i32>,
    pub present_on_disk: Option<bool>,
    pub path: Option<String>,
    pub size: Option<i64>,
    pub parameters: Option<String>,
    pub application_path: Option<String>,
    pub launch_command: Option<String>,
}

impl From<GameData> for PartialGameData {
    fn from(value: GameData) -> Self {
        PartialGameData {
            id: Some(value.id),
            game_id: value.game_id,
            title: Some(value.title),
            date_added: Some(value.date_added),
            sha256: Some(value.sha256),
            crc32: Some(value.crc32),
            present_on_disk: Some(value.present_on_disk),
            path: value.path,
            size: Some(value.size),
            parameters: value.parameters,
            application_path: Some(value.application_path),
            launch_command: Some(value.launch_command),
        }
    }
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let mut stmt = conn.prepare("DELETE FROM game_data WHERE id = ?")?;
    stmt.execute(params![id])?;

    stmt = conn.prepare("UPDATE game SET activeDataId = NULL, activeDataOnDisk = false WHERE activeDataId = ?")?;
    stmt.execute(params![id])?;
    Ok(())
}
