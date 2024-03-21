use chrono::Utc;
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, ValueRef, Value},
    Connection, OptionalExtension, Result,
};
use uuid::Uuid;
use std::{collections::{HashMap, HashSet}, fmt::Display, ops::{Deref, DerefMut}, rc::Rc, vec::Vec};

use crate::{tag::{Tag, self}, platform::{self, PlatformAppPath}, game_data::{GameData, PartialGameData}};

use self::search::{mark_index_dirty, GameSearch, GameSearchRelations};

pub mod search;

#[cfg(feature = "napi")]
use napi::bindgen_prelude::{ToNapiValue, FromNapiValue};

#[derive(Debug, Clone)]
pub struct TagVec (Vec<String>);

#[cfg(feature = "serde")]
impl serde::Serialize for TagVec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let combined = self.0.join(";");
        serializer.serialize_str(&combined)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TagVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TagVecVisitor;

        impl<'de> serde::de::Visitor<'de> for TagVecVisitor {
            type Value = TagVec;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string separated by ;")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let parts: Vec<String> = value.split(';').map(String::from).collect();
                Ok(TagVec(parts))
            }
        }

        deserializer.deserialize_str(TagVecVisitor)
    }
}

impl Deref for TagVec {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TagVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for TagVec {
    fn default() -> Self {
        TagVec (vec![])
    }
}

impl Display for TagVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.join("; ").as_str())?;
        Ok(())
    }
}

#[cfg(feature = "napi")]
impl FromNapiValue for TagVec {
    unsafe fn from_napi_value(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> napi::Result<Self> {
        let mut len = 0;
        napi::sys::napi_get_array_length(env, napi_val, &mut len);

        let mut result = Vec::with_capacity(len as usize);

        for i in 0..len {
            let mut element_value: napi::sys::napi_value = std::ptr::null_mut();
            napi::sys::napi_get_element(env, napi_val, i, &mut element_value);

            // Assuming the elements are N-API strings, we use a utility function to convert them
            let str_length = {
                let mut str_length = 0;
                napi::sys::napi_get_value_string_utf8(
                    env, 
                    element_value, 
                    std::ptr::null_mut(), 
                    0, 
                    &mut str_length
                );
                str_length
            };

            let mut buffer = Vec::with_capacity(str_length as usize + 1);
            let buffer_ptr = buffer.as_mut_ptr() as *mut _;
            napi::sys::napi_get_value_string_utf8(
                env, 
                element_value, 
                buffer_ptr, 
                buffer.capacity(), 
                std::ptr::null_mut()
            );

            buffer.set_len(str_length as usize);
            let string = String::from_utf8_lossy(&buffer).to_string();
            result.push(string);
        }

        Ok(TagVec(result))
    }
}

#[cfg(feature = "napi")]
impl ToNapiValue for TagVec {
    unsafe fn to_napi_value(env: napi::sys::napi_env, val: Self) -> napi::Result<napi::sys::napi_value> {
        let len = val.len();
        let mut js_array: napi::sys::napi_value = std::ptr::null_mut();
        napi::sys::napi_create_array_with_length(env, len, &mut js_array);

        for (i, item) in val.iter().enumerate() {
            let mut js_string: napi::sys::napi_value = std::ptr::null_mut();
            napi::sys::napi_create_string_utf8(env, item.as_ptr() as *const _, item.len(), &mut js_string);

            napi::sys::napi_set_element(env, js_array, i as u32, js_string);
        }

        Ok(js_array)
    }
}

impl IntoIterator for TagVec {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<&str>> for TagVec {
    fn from(vec: Vec<&str>) -> Self {
        let strings: Vec<String> = vec.iter().map(|&s| s.to_string()).collect();
        TagVec (strings)
    }
}

// impl From<Vec<_>> for TagVec {
//     fn from(vec: Vec<_>) -> Self {
//         TagVec(Vec::nmew
//     }
// }

// Custom trait for splitting a string by ";" and removing whitespace
trait FromDelimitedString: Sized {
    fn from_delimited_string(s: &str) -> Result<Self, Box<dyn std::error::Error>>;
}

impl FromDelimitedString for TagVec {
    fn from_delimited_string(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let values: Vec<String> = s
            .split(';')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect();

        Ok(TagVec(values))
    }
}

// Implement FromSql for Vec<String>
impl FromSql for TagVec {
    fn column_result(value: ValueRef) -> Result<Self, FromSqlError> {
        match value {
            ValueRef::Text(_) => {
                let s = value.as_str()?;
                FromDelimitedString::from_delimited_string(s)
                    .map_err(|_| FromSqlError::OutOfRange(0))
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct AdditionalApp {
    pub id: String,
    pub name: String,
    pub application_path: String,
    pub launch_command: String,
    pub auto_run_before: bool,
    pub wait_for_exit: bool,
    pub parent_game_id: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct Game {
    pub id: String,
    pub library: String,
    pub title: String,
    pub alternate_titles: String,
    pub series: String,
    pub developer: String,
    pub publisher: String,
    pub primary_platform: String,
    pub platforms: TagVec,
    pub date_added: String,
    pub date_modified: String,
    pub detailed_platforms: Option<Vec<Tag>>,
    pub legacy_broken: bool,
    pub legacy_extreme: bool,
    pub play_mode: String,
    pub status: String,
    pub notes: String,
    pub tags: TagVec,
    pub detailed_tags: Option<Vec<Tag>>,
    pub source: String,
    pub legacy_application_path: String,
    pub legacy_launch_command: String,
    pub release_date: String,
    pub version: String,
    pub original_description: String,
    pub language: String,
    pub active_data_id: Option<i64>,
    pub active_data_on_disk: bool,
    pub last_played: Option<String>,
    pub playtime: i64,
    pub play_counter: i64,
    pub active_game_config_id: Option<i64>,
    pub active_game_config_owner: Option<String>,
    pub archive_state: i64,
    pub game_data: Option<Vec<GameData>>,
    pub add_apps: Option<Vec<AdditionalApp>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct PartialGame {
    pub id: String,
    pub library: Option<String>,
    pub title: Option<String>,
    pub alternate_titles: Option<String>,
    pub series: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub primary_platform: Option<String>,
    pub platforms: Option<TagVec>,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub legacy_broken: Option<bool>,
    pub legacy_extreme: Option<bool>,
    pub play_mode: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<TagVec>,
    pub source: Option<String>,
    pub legacy_application_path: Option<String>,
    pub legacy_launch_command: Option<String>,
    pub release_date: Option<String>,
    pub version: Option<String>,
    pub original_description: Option<String>,
    pub language: Option<String>,
    pub active_data_id: Option<i64>,
    pub active_data_on_disk: Option<bool>,
    pub last_played: Option<String>,
    pub playtime: Option<i64>,
    pub active_game_config_id: Option<i64>,
    pub active_game_config_owner: Option<String>,
    pub archive_state: Option<i64>,
    pub add_apps: Option<Vec<AdditionalApp>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct GameRedirect {
    pub source_id: String,
    pub dest_id: String,
}

pub fn find_all_ids(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM game")?;

    let ids = stmt.query_map([], |row| {
        row.get(0)
    })?
    .collect::<Result<Vec<String>>>()?;

    Ok(ids)
}

pub fn find(conn: &Connection, id: &str) -> Result<Option<Game>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, alternateTitles, series, developer, publisher, platformsStr, \
        platformName, dateAdded, dateModified, broken, extreme, playMode, status, notes, \
        tagsStr, source, applicationPath, launchCommand, releaseDate, version, \
        originalDescription, language, activeDataId, activeDataOnDisk, lastPlayed, playtime, \
        activeGameConfigId, activeGameConfigOwner, archiveState, library, playCounter \
        FROM game WHERE id = COALESCE((SELECT id FROM game_redirect WHERE sourceId = ?), ?)",
    )?;

    let game_result = stmt
        .query_row(params![id, id], |row| {
            Ok(Game {
                id: row.get(0)?,
                title: row.get(1)?,
                alternate_titles: row.get(2)?,
                series: row.get(3)?,
                developer: row.get(4)?,
                publisher: row.get(5)?,
                platforms: row.get(6)?,
                primary_platform: row.get(7)?,
                date_added: row.get(8)?,
                date_modified: row.get(9)?,
                legacy_broken: row.get(10)?,
                legacy_extreme: row.get(11)?,
                play_mode: row.get(12)?,
                status: row.get(13)?,
                notes: row.get(14)?,
                tags: row.get(15)?,
                source: row.get(16)?,
                legacy_application_path: row.get(17)?,
                legacy_launch_command: row.get(18)?,
                release_date: row.get(19)?,
                version: row.get(20)?,
                original_description: row.get(21)?,
                language: row.get(22)?,
                active_data_id: row.get(23)?,
                active_data_on_disk: row.get(24)?,
                last_played: row.get(25)?,
                playtime: row.get(26)?,
                active_game_config_id: row.get(27)?,
                active_game_config_owner: row.get(28)?,
                archive_state: row.get(29)?,
                library: row.get(30)?,
                play_counter: row.get(31)?,
                detailed_platforms: None,
                detailed_tags: None,
                game_data: None,
                add_apps: None,
            })
        })
        .optional()?; // Converts rusqlite::Error::QueryReturnedNoRows to None

    if let Some(mut game) = game_result {
        game.detailed_platforms = Some(get_game_platforms(conn, id)?);
        game.detailed_tags = Some(get_game_tags(conn, id)?);
        game.game_data = Some(get_game_data(conn, id)?);
        game.add_apps = Some(get_game_add_apps(conn, id)?);
        Ok(Some(game))
    } else {
        Ok(None)
    }
}

pub fn create(conn: &Connection, partial: &PartialGame) -> Result<Game> {
    let mut game: Game = partial.into();

    let mut detailed_tags = vec![];
    let mut detailed_platforms = vec![];

    let tags_copy = game.tags.clone();
    let platforms_copy = game.platforms.clone();
    game.tags = vec![].into();
    game.platforms = vec![].into();

    for name in tags_copy {
        let detailed_tag = tag::find_or_create(conn, &name)?;
        game.tags.push(detailed_tag.name);
        detailed_tags.push(detailed_tag.id);
    }

    for name in platforms_copy {
        let detailed_platform = platform::find_or_create(conn, &name, None)?;
        game.platforms.push(detailed_platform.name);
        detailed_platforms.push(detailed_platform.id);
    }

    conn.execute(
        "INSERT INTO game (id, library, title, alternateTitles, series, developer, publisher, \
         platformName, platformsStr, dateAdded, dateModified, broken, extreme, playMode, status, \
         notes, tagsStr, source, applicationPath, launchCommand, releaseDate, version, \
         originalDescription, language, activeDataId, activeDataOnDisk, lastPlayed, playtime, \
         activeGameConfigId, activeGameConfigOwner, archiveState, orderTitle) VALUES (?, ?, ?, ?, ?, ?, ?, \
         ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '')",
        params![
            &game.id,
            &game.library,
            &game.title,
            &game.alternate_titles,
            &game.series,
            &game.developer,
            &game.publisher,
            &game.primary_platform,
            &game.platforms.join("; "),
            &game.date_added,
            &game.date_modified,
            &game.legacy_broken,
            &game.legacy_extreme,
            &game.play_mode,
            &game.status,
            &game.notes,
            &game.tags.join("; "),
            &game.source,
            &game.legacy_application_path,
            &game.legacy_launch_command,
            &game.release_date,
            &game.version,
            &game.original_description,
            &game.language,
            &game.active_data_id,
            &game.active_data_on_disk,
            &game.last_played,
            &game.playtime,
            &game.active_game_config_id,
            &game.active_game_config_owner,
            &game.archive_state,
        ],
    )?;

    for tag in detailed_tags {
        conn.execute("INSERT OR IGNORE INTO game_tags_tag (gameId, tagId) VALUES (?, ?)", params![game.id, tag])?;
    }

    for platform in detailed_platforms {
        conn.execute("INSERT OR IGNORE INTO game_platforms_platform (gameId, platformId) VALUES (?, ?)", params![game.id, platform])?;
    }

    mark_index_dirty(conn)?;

    Ok(game)
}

pub fn save(conn: &Connection, game: &PartialGame) -> Result<Game> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    let existing_game_result = find(conn, game.id.as_str())?;
    if let Some(mut existing_game) = existing_game_result {
        existing_game.apply_partial(game);

        // Process  any tag and platform changes
        let tags_copy = existing_game.tags.clone();
        let platforms_copy = existing_game.platforms.clone();
        let mut detailed_tags_copy: Vec<Tag> = vec![];
        let mut detailed_platforms_copy: Vec<Tag> = vec![];
        existing_game.tags = vec![].into();
        existing_game.platforms = vec![].into();

        for name in tags_copy {
            let detailed_tag = tag::find_or_create(conn, &name)?;
            detailed_tags_copy.push(detailed_tag.clone());
            existing_game.tags.push(detailed_tag.name);
        }

        for name in platforms_copy {
            let detailed_platform = platform::find_or_create(conn, &name, None)?;
            detailed_platforms_copy.push(detailed_platform.clone());
            existing_game.platforms.push(detailed_platform.name);
        }

        // Update relations in database
        let tag_ids: Vec<i64> = detailed_tags_copy.iter().map(|t| t.id).collect::<Vec<i64>>();
        let tag_values = Rc::new(tag_ids.iter().copied().map(Value::from).collect::<Vec<Value>>());
        let mut stmt = conn.prepare("DELETE FROM game_tags_tag WHERE gameId = ? AND tagId NOT IN rarray(?)")?;
        stmt.execute(params![existing_game.id.as_str(), tag_values]).map(|changes| changes as usize)?;
        for tag_id in tag_ids {
            stmt = conn.prepare("INSERT OR IGNORE INTO game_tags_tag (gameId, tagId) VALUES (?, ?)")?;
            stmt.execute(params![existing_game.id.as_str(), tag_id])?;
        }

        let platform_ids: Vec<i64> = detailed_platforms_copy.iter().map(|t| t.id).collect::<Vec<i64>>();
        let platform_values = Rc::new(platform_ids.iter().copied().map(Value::from).collect::<Vec<Value>>());
        let mut stmt = conn.prepare("DELETE FROM game_platforms_platform WHERE gameId = ? AND platformId NOT IN rarray(?)")?;
        stmt.execute(params![existing_game.id.as_str(), platform_values]).map(|changes| changes as usize)?;
        for platform_id in platform_ids {
            stmt = conn.prepare("INSERT OR IGNORE INTO game_platforms_platform (gameId, platformId) VALUES (?, ?)")?;
            stmt.execute(params![existing_game.id.as_str(), platform_id])?;
        }


        // Write back the changes to the database
        conn.execute(
            "UPDATE game SET library = ?, title = ?, alternateTitles = ?, series = ?, developer = ?, publisher = ?, \
             platformName = ?, platformsStr = ?, dateAdded = ?, dateModified = ?, broken = ?, \
             extreme = ?, playMode = ?, status = ?, notes = ?, tagsStr = ?, source = ?, \
             applicationPath = ?, launchCommand = ?, releaseDate = ?, version = ?, \
             originalDescription = ?, language = ?, activeDataId = ?, activeDataOnDisk = ?, \
             lastPlayed = ?, playtime = ?, playCounter = ?, activeGameConfigId = ?, activeGameConfigOwner = ?, \
             archiveState = ? WHERE id = ?",
            params![
                &existing_game.library,
                &existing_game.title,
                &existing_game.alternate_titles,
                &existing_game.series,
                &existing_game.developer,
                &existing_game.publisher,
                &existing_game.primary_platform,
                &existing_game.platforms.join("; "),
                &existing_game.date_added,
                &existing_game.date_modified,
                &existing_game.legacy_broken,
                &existing_game.legacy_extreme,
                &existing_game.play_mode,
                &existing_game.status,
                &existing_game.notes,
                &existing_game.tags.join("; "),
                &existing_game.source,
                &existing_game.legacy_application_path,
                &existing_game.legacy_launch_command,
                &existing_game.release_date,
                &existing_game.version,
                &existing_game.original_description,
                &existing_game.language,
                &existing_game.active_data_id,
                &existing_game.active_data_on_disk,
                &existing_game.last_played,
                &existing_game.playtime,
                &existing_game.play_counter,
                &existing_game.active_game_config_id,
                &existing_game.active_game_config_owner,
                &existing_game.archive_state,
                &existing_game.id,
            ],
        )?;



        existing_game.detailed_platforms = get_game_platforms(conn, &existing_game.id)?.into();
        existing_game.detailed_tags = get_game_tags(conn, &existing_game.id)?.into();
        existing_game.game_data = get_game_data(conn, &existing_game.id)?.into();

        mark_index_dirty(conn)?;

        Ok(existing_game)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {    
    let mut stmt = "DELETE FROM game WHERE id = ?";
    conn.execute(stmt, params![id])?;

    stmt = "DELETE FROM additional_app WHERE parentGameId = ?";
    conn.execute(stmt, params![id])?;

    stmt = "DELETE FROM game_tags_tag WHERE gameId = ?";
    conn.execute(stmt, params![id])?;

    stmt = "DELETE FROM game_platforms_platform WHERE gameId = ?";
    conn.execute(stmt, params![id])?;

    Ok(())
}

pub fn count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM game", (), |row| row.get::<_, i64>(0))
}

fn get_game_platforms(conn: &Connection, id: &str) -> Result<Vec<Tag>> {
    let mut platform_stmt = conn.prepare(
        "SELECT p.id, p.description, pa.name, p.dateModified FROM platform p
         INNER JOIN game_platforms_platform gpp ON gpp.platformId = p.id
         INNER JOIN platform_alias pa ON p.primaryAliasId = pa.id
         WHERE gpp.gameId = ?",
    )?;

    let platform_iter = platform_stmt.query_map(params![id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            description: row.get(1)?,
            name: row.get(2)?,
            date_modified: row.get(3)?,
            category: None,
            aliases: vec![],
        })
    })?;

    let mut platforms: Vec<Tag> = vec![];

    for platform_result in platform_iter {
        let mut platform = platform_result?;

        // Query for the aliases of the platform
        let mut platform_aliases_stmt =
            conn.prepare("SELECT pa.name FROM platform_alias pa WHERE pa.platformId = ?")?;

        let aliases_iter = platform_aliases_stmt
            .query_map(params![platform.id], |row| Ok(row.get::<_, String>(0)?))?;

        // Collect aliases into the platform's aliases vector
        for alias_result in aliases_iter {
            platform.aliases.push(alias_result?);
        }

        platforms.push(platform);
    }

    Ok(platforms)
}

fn get_game_tags(conn: &Connection, id: &str) -> Result<Vec<Tag>> {
    let mut tag_stmt = conn.prepare(
        "SELECT t.id, t.description, ta.name, t.dateModified, tc.name FROM tag t
         INNER JOIN game_tags_tag gtt ON gtt.tagId = t.id
         INNER JOIN tag_alias ta ON t.primaryAliasId = ta.id
         INNER JOIN tag_category tc ON t.categoryId = tc.id
         WHERE gtt.gameId = ?",
    )?;

    let tag_iter = tag_stmt.query_map(params![id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            description: row.get(1)?,
            name: row.get(2)?,
            date_modified: row.get(3)?,
            category: row.get(4)?,
            aliases: vec![],
        })
    })?;

    let mut tags: Vec<Tag> = vec![];

    for tag_result in tag_iter {
        let mut tag = tag_result?;

        // Query for the aliases of the platform
        let mut tag_aliases_stmt =
            conn.prepare("SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;

        let aliases_iter =
            tag_aliases_stmt.query_map(params![tag.id], |row| Ok(row.get::<_, String>(0)?))?;

        // Collect aliases into the platform's aliases vector
        for alias_result in aliases_iter {
            tag.aliases.push(alias_result?);
        }

        tags.push(tag);
    }

    Ok(tags)
}

pub fn get_game_data(conn: &Connection, id: &str) -> Result<Vec<GameData>> {
    let mut game_data: Vec<GameData> = vec![];

    let mut game_data_stmt = conn.prepare("
        SELECT id, title, dateAdded, sha256, crc32, presentOnDisk,
        path, size, parameters, applicationPath, launchCommand
        FROM game_data
        WHERE gameId = ?
    ")?;

    let rows = game_data_stmt.query_map(params![id], |row| {
        Ok(GameData {
            id: row.get(0)?,
            game_id: id.to_owned(),
            title: row.get(1)?,
            date_added: row.get(2)?,
            sha256: row.get(3)?,
            crc32: row.get(4)?,
            present_on_disk: row.get(5)?,
            path: row.get(6)?,
            size: row.get(7)?,
            parameters: row.get(8)?,
            application_path: row.get(9)?,
            launch_command: row.get(10)?,
        })
    })?;

    for result in rows {
        game_data.push(result?);
    }

    Ok(game_data)
}

fn get_game_add_apps(conn: &Connection, game_id: &str) -> Result<Vec<AdditionalApp>> {
    let mut add_app_stmt = conn.prepare(
        "SELECT id, name, applicationPath, launchCommand, autoRunBefore, waitForExit
        FROM additional_app WHERE parentGameId = ?"
    )?;

    let mut add_apps: Vec<AdditionalApp> = vec![];

    let add_app_iter = add_app_stmt.query_map(params![game_id], |row| {
        Ok(AdditionalApp {
            id: row.get(0)?,
            parent_game_id: game_id.to_owned(),
            name: row.get(1)?,
            application_path: row.get(2)?,
            launch_command: row.get(3)?,
            auto_run_before: row.get(4)?,
            wait_for_exit: row.get(5)?,
        })
    })?;

    for add_app in add_app_iter {
        add_apps.push(add_app?);
    }

    Ok(add_apps)
}

pub fn find_game_data_by_id(conn: &Connection, id: i64) -> Result<Option<GameData>> {
    let mut game_data_stmt = conn.prepare("
        SELECT gameId, title, dateAdded, sha256, crc32, presentOnDisk,
        path, size, parameters, applicationPath, launchCommand
        FROM game_data
        WHERE id = ?
    ")?;

    Ok(game_data_stmt.query_row(params![id], |row| {
        Ok(GameData {
            id: id.to_owned(),
            game_id: row.get(0)?,
            title: row.get(1)?,
            date_added: row.get(2)?,
            sha256: row.get(3)?,
            crc32: row.get(4)?,
            present_on_disk: row.get(5)?,
            path: row.get(6)?,
            size: row.get(7)?,
            parameters: row.get(8)?,
            application_path: row.get(9)?,
            launch_command: row.get(10)?,
        })
    }).optional()?)
}

pub fn create_game_data(conn: &Connection, partial: &PartialGameData) -> Result<GameData> {
    // Make sure game exists
    let game = find(conn, &partial.game_id)?;
    if game.is_none() {
        println!("{} missing", &partial.game_id);
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }

    let mut game_data: GameData = partial.into();
    
    let mut stmt = conn.prepare("INSERT INTO game_data (gameId, title, dateAdded, sha256, crc32, presentOnDisk
        , path, size, parameters, applicationPath, launchCommand)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id")?;
    let game_data_id: i64 = stmt.query_row(params![
        &game_data.game_id,
        &game_data.title,
        &game_data.date_added,
        &game_data.sha256,
        &game_data.crc32,
        &game_data.present_on_disk,
        &game_data.path,
        &game_data.size,
        &game_data.parameters,
        &game_data.application_path,
        &game_data.launch_command,
    ], |row| row.get(0))?;

    game_data.id = game_data_id;
    Ok(game_data)
}

pub fn save_game_data(conn: &Connection, partial: &PartialGameData) -> Result<GameData> {
    let game_data: GameData = partial.into();
    
    let mut stmt = conn.prepare("UPDATE game_data
        SET gameId = ?, title = ?, dateAdded = ?, sha256 = ?, crc32 = ?, presentOnDisk = ?,
        path = ?, size = ?, parameters = ?, applicationPath = ?, launchCommand = ? WHERE id = ?")?;
    stmt.execute(params![
        &game_data.game_id,
        &game_data.title,
        &game_data.date_added,
        &game_data.sha256,
        &game_data.crc32,
        &game_data.present_on_disk,
        &game_data.path,
        &game_data.size,
        &game_data.parameters,
        &game_data.application_path,
        &game_data.launch_command,
        &game_data.id,
    ])?;

    let res = find_game_data_by_id(conn, game_data.id)?;
    match res {
        Some(r) => Ok(r),
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

pub fn find_with_tag(conn: &Connection, tag: &str) -> Result<Vec<Game>> {
    let mut search = GameSearch::default();
    search.load_relations = GameSearchRelations {
        tags: true,
        platforms: true,
        game_data: true,
        add_apps: true,
    };
    search.filter.exact_whitelist.tags = Some(vec![tag.to_owned()]);
    search.limit = 9999999999;
    search::search(conn, &search)
}

pub fn find_libraries(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT library FROM game")?;
    let libraries_iter = stmt.query_map((), |row| row.get(0))?;

    let mut libraries = vec![];

    for library in libraries_iter {
        libraries.push(library?);
    }

    Ok(libraries)
}

pub fn find_statuses(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT status FROM game")?;
    let status_iter = stmt.query_map((), |row| {
        let value: String = row.get(0)?;
        Ok(value)
    })?;

    let mut statuses = HashSet::new();

    for status in status_iter {
        if let Ok(status) = status {

            status.split(';').for_each(|v| { statuses.insert(v.trim().to_string()); });
        }
    }

    Ok(statuses.into_iter().collect())
}

pub fn find_play_modes(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT playMode FROM game")?;
    let play_modes_iter = stmt.query_map((), |row| {
        let value: String = row.get(0)?;
        Ok(value)
    })?;

    let mut play_modes = HashSet::new();

    for play_mode in play_modes_iter {
        if let Ok(play_mode) = play_mode {

            play_mode.split(';').for_each(|v| { play_modes.insert(v.trim().to_string()); });
        }
    }

    Ok(play_modes.into_iter().collect())
}

pub fn find_application_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("
    SELECT COUNT(*) as games_count, applicationPath FROM (
        SELECT applicationPath FROM game WHERE applicationPath != ''
        UNION ALL
        SELECT applicationPath FROM game_data WHERE applicationPath != ''
    ) GROUP BY applicationPath ORDER BY games_count DESC")?;
    let ap_iter = stmt.query_map((), |row| row.get(1))?;

    let mut app_paths = vec![];

    for app_path in ap_iter {
        app_paths.push(app_path?);
    }

    Ok(app_paths)
}

pub fn find_platform_app_paths(conn: &Connection) -> Result<HashMap<String, Vec<PlatformAppPath>>> {
    let mut suggestions = HashMap::new();
    let platforms = platform::find(conn)?;

    for platform in platforms {
        let mut stmt = conn.prepare("
        SELECT COUNT(*) as games_count, applicationPath FROM (
            SELECT applicationPath FROM game WHERE applicationPath != '' AND game.id IN (
                SELECT gameId FROM game_platforms_platform WHERE platformId = ?
            )
            UNION ALL
            SELECT applicationPath FROM game_data WHERE applicationPath != '' AND game_data.gameId IN (
                SELECT gameId FROM game_platforms_platform WHERE platformId = ?
            )
        ) GROUP BY applicationPath ORDER BY games_count DESC")?;

        let results = stmt.query_map(params![platform.id, platform.id], |row| {
            Ok(PlatformAppPath {
                app_path: row.get(1)?,
                count: row.get(0)?,
            })
        })?;

        let mut platform_list = vec![];

        for app_path in results {
            platform_list.push(app_path?);
        }

        suggestions.insert(platform.name, platform_list);
    }

    Ok(suggestions)
}

pub fn find_add_app_by_id(conn: &Connection, id: &str) -> Result<Option<AdditionalApp>> {
    let mut stmt = conn.prepare("SELECT name, applicationPath, launchCommand, autoRunBefore,
        waitForExit, parentGameId FROM additional_app WHERE id = ?")?;

    stmt.query_row(params![id], |row| {
        Ok(AdditionalApp{
            id: id.to_owned(),
            name: row.get(0)?,
            application_path: row.get(1)?,
            launch_command: row.get(2)?,
            auto_run_before: row.get(3)?,
            wait_for_exit: row.get(4)?,
            parent_game_id: row.get(5)?
        })
    }).optional()
}

pub fn create_add_app(conn: &Connection, add_app: &mut AdditionalApp) -> Result<()> {
    let id = conn.query_row("INSERT INTO additional_app (
        id, applicationPath, launchCommand, name, parentGameId, autoRunBefore, waitForExit
    ) VALUES (?, ?, ?, ?, ?, ? , ?) RETURNING id", params![add_app.id, add_app.application_path, add_app.launch_command,
    add_app.name, add_app.parent_game_id, add_app.auto_run_before, add_app.wait_for_exit], |row| row.get::<_, String>(0))?;
    add_app.id = id;
    Ok(())
}

pub fn add_playtime(conn: &Connection, game_id: &str, seconds: i64) -> Result<()> {
    let mut game = match find(conn, game_id)? {
        Some(g) => g,
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    };

    game.play_counter += 1;
    game.playtime += seconds;
    game.last_played = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

    save(conn, &(game.into()))?;
    Ok(())
}

pub fn clear_playtime_tracking(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("UPDATE game SET playtime = 0, play_counter = 0, last_played = NULL")?;
    stmt.execute(())?;
    Ok(())
}

pub fn clear_playtime_tracking_by_id(conn: &Connection, game_id: &str) -> Result<()> {
    let mut stmt = conn.prepare("UPDATE game SET playtime = 0, play_counter = 0, last_played = NULL WHERE id = ?")?;
    stmt.execute(params![game_id])?;
    Ok(())
}

pub fn force_active_data_most_recent(conn: &Connection) -> Result<()> {
    conn.execute("UPDATE game
    SET activeDataId = (SELECT game_data.id FROM game_data WHERE game.id = game_data.gameId ORDER BY game_data.dateAdded DESC LIMIT 1)
    WHERE game.activeDataId = -1", ())?;
    Ok(())
}

pub fn find_redirects(conn: &Connection) -> Result<Vec<GameRedirect>> {
    let mut redirects = vec![];

    let mut stmt = conn.prepare("SELECT sourceId, id, dateAdded FROM game_redirect")?;
    let redirects_iter = stmt.query_map((), |row| Ok(GameRedirect{
        source_id: row.get(0)?,
        dest_id: row.get(1)?
    }))?;

    for r in redirects_iter {
        redirects.push(r?);
    }

    Ok(redirects)
}

pub fn create_redirect(conn: &Connection, src_id: &str, dest_id: &str) -> Result<()> {
    conn.execute("INSERT OR IGNORE INTO game_redirect (sourceId, id) VALUES (?, ?)", params![src_id, dest_id])?;
    Ok(())
}

pub fn delete_redirect(conn: &Connection, src_id: &str, dest_id: &str) -> Result<()> {
    conn.execute("DELETE FROM game_redirect WHERE sourceId = ? AND id = ?", params![src_id, dest_id])?;
    Ok(())
}

impl Default for PartialGame {
    fn default() -> Self {
        PartialGame {
            id: String::from(""),
            library: None,
            title: None,
            alternate_titles: None,
            series: None,
            developer: None,
            publisher: None,
            primary_platform: None,
            platforms: None,
            date_added: None,
            date_modified: None,
            legacy_broken: None,
            legacy_extreme: None,
            play_mode: None,
            status: None,
            notes: None,
            tags: None,
            source: None,
            legacy_application_path: None,
            legacy_launch_command: None,
            release_date: None,
            version: None,
            original_description: None,
            language: None,
            active_data_id: None,
            active_data_on_disk: None,
            last_played: None,
            playtime: None,
            active_game_config_id: None,
            active_game_config_owner: None,
            archive_state: None,
            add_apps: None,
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Game {
            id: Uuid::new_v4().to_string(),
            library: String::from("arcade"),
            title: String::default(),
            alternate_titles: String::default(),
            series: String::default(),
            developer: String::default(),
            publisher: String::default(),
            primary_platform: String::default(),
            platforms: TagVec::default(),
            date_added: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            date_modified: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            detailed_platforms: None,
            legacy_broken: false,
            legacy_extreme: false,
            play_mode: String::default(),
            status: String::default(),
            notes: String::default(),
            tags: TagVec::default(),
            detailed_tags: None,
            source: String::default(),
            legacy_application_path: String::default(),
            legacy_launch_command: String::default(),
            release_date: String::default(),
            version: String::default(),
            original_description: String::default(),
            language: String::default(),
            active_data_id: None,
            active_data_on_disk: false,
            last_played: None,
            playtime: 0,
            play_counter: 0,
            active_game_config_id: None,
            active_game_config_owner: None,
            archive_state: 0,
            game_data: None,
            add_apps: None,
        }
    }
}

impl Game {
    fn apply_partial(&mut self, source: &PartialGame) {
        if source.id == "" {
            self.id = Uuid::new_v4().to_string();
        } else {
            self.id = source.id.clone();
        }

        if let Some(library) = source.library.clone() {
            self.library = library;
        }

        if let Some(title) = source.title.clone() {
            self.title = title;
        }
    
        if let Some(alternate_titles) = source.alternate_titles.clone() {
            self.alternate_titles = alternate_titles;
        }
    
        if let Some(series) = source.series.clone() {
            self.series = series;
        }
    
        if let Some(developer) = source.developer.clone() {
            self.developer = developer;
        }
    
        if let Some(publisher) = source.publisher.clone() {
            self.publisher = publisher;
        }

        if let Some(platforms) = source.platforms.clone() {
            self.platforms = platforms;
        }
    
        if let Some(platform) = source.primary_platform.clone() {
            // Make sure platforms always includes the primary platform
            if !self.platforms.contains(&platform) {
                self.platforms.push(platform.clone());
            }

            self.primary_platform = platform;
        }
    
        if let Some(date_added) = source.date_added.clone() {
            self.date_added = date_added;
        }
    
        if let Some(date_modified) = source.date_modified.clone() {
            self.date_modified = date_modified;
        }
    
        if let Some(legacy_broken) = source.legacy_broken {
            self.legacy_broken = legacy_broken;
        }
    
        if let Some(legacy_extreme) = source.legacy_extreme {
            self.legacy_extreme = legacy_extreme;
        }
    
        if let Some(play_mode) = source.play_mode.clone() {
            self.play_mode = play_mode;
        }
    
        if let Some(status) = source.status.clone() {
            self.status = status;
        }
    
        if let Some(notes) = source.notes.clone() {
            self.notes = notes;
        }
    
        if let Some(tags) = source.tags.clone() {
            self.tags = tags;
        }
    
        if let Some(source) = source.source.clone() {
            self.source = source;
        }
    
        if let Some(legacy_application_path) = source.legacy_application_path.clone() {
            self.legacy_application_path = legacy_application_path;
        }
    
        if let Some(legacy_launch_command) = source.legacy_launch_command.clone() {
            self.legacy_launch_command = legacy_launch_command;
        }
    
        if let Some(release_date) = source.release_date.clone() {
            self.release_date = release_date;
        }
    
        if let Some(version) = source.version.clone() {
            self.version = version;
        }
    
        if let Some(original_description) = source.original_description.clone() {
            self.original_description = original_description;
        }
    
        if let Some(language) = source.language.clone() {
            self.language = language;
        }
    
        if let Some(active_data_id) = source.active_data_id {
            self.active_data_id = Some(active_data_id);
        }
    
        if let Some(active_data_on_disk) = source.active_data_on_disk {
            self.active_data_on_disk = active_data_on_disk;
        }
    
        if let Some(last_played) = source.last_played.clone() {
            self.last_played = Some(last_played);
        }
    
        if let Some(playtime) = source.playtime {
            self.playtime = playtime;
        }
    
        if let Some(active_game_config_id) = source.active_game_config_id {
            self.active_game_config_id = Some(active_game_config_id);
        }
    
        if let Some(active_game_config_owner) = source.active_game_config_owner.clone() {
            self.active_game_config_owner = Some(active_game_config_owner);
        }
    
        if let Some(archive_state) = source.archive_state {
            self.archive_state = archive_state;
        }
    }
}

impl From<&PartialGame> for Game {
    fn from(source: &PartialGame) -> Self {
        let mut game = Game::default();
        game.apply_partial(source);
        game
    }
}

impl From<Game> for PartialGame {
    fn from(game: Game) -> Self {
        let mut new_plats = game.platforms.clone();
        // Make sure game.platform is present in the vec. If not, add it
        if !new_plats.contains(&game.primary_platform) {
            new_plats.push(game.primary_platform.clone());
        }

        PartialGame {
            id: game.id,
            library: Some(game.library),
            title: Some(game.title),
            alternate_titles: Some(game.alternate_titles),
            series: Some(game.series),
            developer: Some(game.developer),
            publisher: Some(game.publisher),
            primary_platform: Some(game.primary_platform),
            platforms: Some(new_plats),
            date_added: Some(game.date_added),
            date_modified: Some(game.date_modified),
            legacy_broken: Some(game.legacy_broken),
            legacy_extreme: Some(game.legacy_extreme),
            play_mode: Some(game.play_mode),
            status: Some(game.status),
            notes: Some(game.notes),
            tags: Some(game.tags),
            source: Some(game.source),
            legacy_application_path: Some(game.legacy_application_path),
            legacy_launch_command: Some(game.legacy_launch_command),
            release_date: Some(game.release_date),
            version: Some(game.version),
            original_description: Some(game.original_description),
            language: Some(game.language),
            active_data_id: game.active_data_id,
            active_data_on_disk: Some(game.active_data_on_disk),
            last_played: game.last_played,
            playtime: Some(game.playtime),
            active_game_config_id: game.active_game_config_id,
            active_game_config_owner: game.active_game_config_owner,
            archive_state: Some(game.archive_state),
            add_apps: game.add_apps,
        }
    }
}

impl GameData {
    fn apply_partial(&mut self, value: &PartialGameData) {
        if let Some(id) = value.id {
            self.id = id;
        }

        if let Some(title) = value.title.clone() {
            self.title = title;
        }

        if let Some(data_added) = value.date_added.clone() {
            self.date_added = data_added;
        }

        if let Some(sha256) = value.sha256.clone() {
            self.sha256 = sha256;
        }

        if let Some(crc32) = value.crc32 {
            self.crc32 = crc32;
        }

        if let Some(size) = value.size {
            self.size = size;
        }

        if let Some(present_on_disk) = value.present_on_disk {
            self.present_on_disk = present_on_disk;
        }

        if let Some(path) = value.path.clone() {
            self.path = Some(path);
        }
        
        if let Some(parameters) = value.parameters.clone() {
            self.parameters = Some(parameters);
        }
    
        if let Some(application_path) = value.application_path.clone() {
            self.application_path = application_path;
        }

        if let Some(launch_command) = value.launch_command.clone() {
            self.launch_command = launch_command;
        }

    }
}

impl Default for GameData {
    fn default() -> Self {
        GameData {
            id: -1,
            game_id: "".to_owned(),
            title: "".to_owned(),
            date_added: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            sha256: "".to_owned(),
            crc32: 0,
            size: 0,
            present_on_disk: false,
            path: None,
            parameters: None,
            application_path: "".to_owned(),
            launch_command: "".to_owned(),
        }
    }
}

impl From<&PartialGameData> for GameData {
    fn from(value: &PartialGameData) -> Self {
        let mut data = GameData {
            id: -1,
            game_id: value.game_id.clone(),
            ..Default::default()
        };

        data.apply_partial(value);

        data
    }
}