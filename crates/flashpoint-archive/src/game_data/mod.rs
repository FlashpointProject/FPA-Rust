use chrono::NaiveDateTime;

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameData {
    pub id: i64,
    pub game_id: String,
    pub title: String,
    pub date_added: NaiveDateTime,
    pub sha256: String,
    pub crc32: i32,
    pub present_on_disk: bool,
    pub path: Option<String>,
    pub size: i64,
    pub parameters: Option<String>,
    pub application_path: String,
    pub launch_command: String,
}