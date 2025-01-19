use std::rc::Rc;

use rusqlite::types::{ToSqlOutput, Value};
use rusqlite::{params, Connection, ToSql};
use snafu::ResultExt;
use uuid::Uuid;

use crate::game::GameRedirect;
use crate::{error, game, tag, tag_category};
use crate::error::Result;
use crate::game::search::mark_index_dirty;
use crate::platform;

#[derive(Debug, Clone)]
pub struct SqlVec<T> (pub Vec<T>);

impl ToSql for SqlVec<i64> {
    fn to_sql(&self) -> std::result::Result<ToSqlOutput<'_>, rusqlite::Error> {
        let v = Rc::new(self.0.iter().map(|v| Value::from(v.clone())).collect::<Vec<Value>>());
        Ok(ToSqlOutput::Array(v))
    }
}

impl ToSql for SqlVec<String> {
    fn to_sql(&self) -> std::result::Result<ToSqlOutput<'_>, rusqlite::Error> {
        let v = Rc::new(self.0.iter().map(|v| Value::from(v.clone())).collect::<Vec<Value>>());
        Ok(ToSqlOutput::Array(v))
    }
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteDeletedGamesRes {
    pub games: Vec<RemoteDeletedGame>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteDeletedGame {
    pub id: String,
    pub date_modified: String,
    pub reason: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteGamesRes {
    pub games: Vec<RemoteGame>,
    pub add_apps: Vec<RemoteAddApp>,
    pub game_data: Vec<RemoteGameData>,
    pub tag_relations: Vec<Vec<String>>,
    pub platform_relations: Vec<Vec<String>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteGameData {
    pub game_id: String,
    pub title: String,
    pub date_added: String,
    pub sha_256: String,
    pub crc_32: u32,
    pub size: i64,
    pub parameters: Option<String>,
    pub application_path: String,
    pub launch_command: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteAddApp {
    pub name: String,
    pub application_path: String,
    pub launch_command: String,
    pub wait_for_exit: bool,
    pub auto_run_before: bool,
    pub parent_game_id: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteGame {
    pub id: String,
    pub title: String,
    pub alternate_titles: String,
    pub series: String,
    pub developer: String,
    pub publisher: String,
    pub date_added: String,
    pub date_modified: String,
    pub play_mode: String,
    pub status: String,
    pub notes: String,
    pub source: String,
    pub application_path: String,
    pub launch_command: String,
    pub release_date: String,
    pub version: String,
    pub original_description: String,
    pub language: String,
    pub library: String,
    pub platform_name: String,
    pub archive_state: i32,
    pub ruffle_support: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteCategory {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemoteTag {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub category: String,
    pub date_modified: String,
    pub aliases: Vec<String>,
    pub deleted: bool,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct RemotePlatform {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub date_modified: String,
    pub aliases: Vec<String>,
    pub deleted: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug)]
pub struct Alias {
    id: i64,
    value: String,
}

pub fn apply_platforms(conn: &Connection, platforms: Vec<RemotePlatform>) -> Result<()> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn).context(error::SqliteSnafu)?;
    
    // Create a list of Alias structs from the aliases
    let changed_aliases: Vec<Alias> = platforms.iter()
        .flat_map(|cur| cur.aliases.iter().map(move |alias| Alias { id: cur.id, value: alias.clone() }))
        .collect();

    let existing_platforms = platform::find(conn).context(error::SqliteSnafu)?;
    let existing_ids: std::collections::HashSet<i64> = existing_platforms.iter().map(|p| p.id).collect();

    // Delete old platform aliases
    let changed_alias_names = SqlVec(changed_aliases.iter().map(|a| a.value.clone()).collect::<Vec<String>>());
    conn.execute("DELETE FROM platform_alias WHERE name IN rarray(?)", params![changed_alias_names]).context(error::SqliteSnafu)?;

    let mut update_platform_stmt = conn.prepare("UPDATE platform SET dateModified = ?, primaryAliasId = (SELECT id FROM platform_alias WHERE name = ?), description = ? WHERE id = ?").context(error::SqliteSnafu)?;
    let mut insert_platform_stmt = conn.prepare("INSERT INTO platform (id, dateModified, primaryAliasId, description) VALUES (?, ?, (SELECT id FROM platform_alias WHERE name = ?), ?)").context(error::SqliteSnafu)?;
    let mut delete_platform_alias_stmt = conn.prepare("DELETE FROM platform_alias WHERE platformId = ?").context(error::SqliteSnafu)?;
    let mut delete_platform_stmt = conn.prepare("DELETE FROM platform WHERE id = ?").context(error::SqliteSnafu)?;

    // Insert new ones
    let mut insert_alias_stmt = conn.prepare("INSERT INTO platform_alias (platformId, name) VALUES (?, ?)").context(error::SqliteSnafu)?;
    for alias in changed_aliases {
        insert_alias_stmt.execute(params![alias.id, alias.value]).context(error::SqliteSnafu)?;
    }

    // Handle deleted platforms
    let deleted_platform_ids = SqlVec(platforms.iter().filter(|p| existing_ids.contains(&p.id) && p.deleted).map(|p| p.id).collect::<Vec<i64>>());
    // Remove from game platformsStr
    conn.execute("UPDATE game
    SET platformsStr = (
        SELECT IFNULL(string_agg(pa.name, '; '), '')
        FROM game_platforms_platform gpp
        JOIN platform p ON gpp.platformId = p.id
        JOIN platform_alias pa ON p.primaryAliasId = pa.id
        WHERE gpp.gameId = game.id AND p.id NOT IN rarray(?)
    ) WHERE game.id IN (
        SELECT gameId FROM game_platforms_platform WHERE platformId IN rarray(?) 
    )", params![deleted_platform_ids, deleted_platform_ids]).context(error::SqliteSnafu)?;
    // Remove from game platformName
    conn.execute("UPDATE game
    SET platformName = 'BROKEN'
    WHERE platformName IN (
        SELECT name FROM platform_alias WHERE platformId IN rarray(?)   
    )", params![deleted_platform_ids]).context(error::SqliteSnafu)?;
    // Remove all data
    conn.execute("DELETE FROM game_platforms_platform WHERE platformId IN rarray(?)", params![deleted_platform_ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM platform_alias WHERE platformId IN rarray(?)", params![deleted_platform_ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM platform WHERE id IN rarray(?)", params![deleted_platform_ids]).context(error::SqliteSnafu)?;

    // Handle updated platforms
    for platform in platforms.iter().filter(|p| existing_ids.contains(&p.id) && !p.deleted) {
        update_platform_stmt.execute(params![platform.date_modified, platform.name, platform.description, platform.id]).context(error::SqliteSnafu)?;
    }

    // Handle new platforms
    for platform in platforms.iter().filter(|p| !existing_ids.contains(&p.id) && !p.deleted) {
        // Clean up any 'loose' rows
        delete_platform_alias_stmt.execute(params![platform.id]).context(error::SqliteSnafu)?;
        delete_platform_stmt.execute(params![platform.id]).context(error::SqliteSnafu)?;

        // Insert new platform entry (above already added aliases)
        for alias in &platform.aliases {
            insert_alias_stmt.execute(params![platform.id, &alias]).context(error::SqliteSnafu)?;
        }
        insert_platform_stmt.execute(params![platform.id, platform.date_modified, platform.name, platform.description]).context(error::SqliteSnafu)?;
    }

    Ok(())
}

pub fn apply_categories(conn: &Connection, categories: Vec<RemoteCategory>) -> Result<()> {
    let existing_categories = tag_category::find(conn).context(error::SqliteSnafu)?;
    let existing_ids: std::collections::HashSet<i64> = existing_categories.iter().map(|p| p.id).collect();

    let mut update_stmt = conn.prepare("UPDATE tag_category SET description = ?, color = ?, name = ? WHERE id = ?").context(error::SqliteSnafu)?;
    let mut insert_stmt = conn.prepare("INSERT INTO tag_category (id, description, color, name) VALUES (?, ?, ?, ?)").context(error::SqliteSnafu)?;

    // Handle updated platforms
    for cat in categories.iter().filter(|p| existing_ids.contains(&p.id)) {
        update_stmt.execute(params![cat.description, cat.color, cat.name, cat.id]).context(error::SqliteSnafu)?;
    }

    // Handle new platforms
    for cat in categories.iter().filter(|p| !existing_ids.contains(&p.id)) {
        insert_stmt.execute(params![cat.id, cat.description, cat.color, cat.name]).context(error::SqliteSnafu)?;
    }

    Ok(())
}

pub fn apply_tags(conn: &Connection, tags: Vec<RemoteTag>) -> Result<()> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn).context(error::SqliteSnafu)?;
    
    // Create a list of Alias structs from the aliases
    let changed_aliases: Vec<Alias> = tags.iter()
        .flat_map(|cur| cur.aliases.iter().map(move |alias| Alias { id: cur.id, value: alias.clone() }))
        .collect();

    let changed_ids: Vec<i64> = tags.iter().map(|cur| cur.id).collect();

    let existing_tags = tag::find(conn).context(error::SqliteSnafu)?;
    let existing_ids: std::collections::HashSet<i64> = existing_tags.iter().map(|p| p.id).collect();

    // Delete old tag aliases
    let changed_alias_names = SqlVec(changed_aliases.iter().map(|a| a.value.clone()).collect::<Vec<String>>());
    conn.execute("DELETE FROM tag_alias WHERE name IN rarray(?)", params![changed_alias_names]).context(error::SqliteSnafu)?;

    // Clear aliases on all changed tags
    let changed_ids_vec = SqlVec(changed_ids);
    conn.execute("DELETE FROM tag_alias WHERE tagId IN rarray(?)", params![changed_ids_vec]).context(error::SqliteSnafu)?;

    let mut update_tag_stmt = conn.prepare("UPDATE tag SET dateModified = ?, primaryAliasId = (SELECT id FROM tag_alias WHERE name = ?), description = ?, categoryId = (SELECT id FROM tag_category WHERE name = ?) WHERE id = ?").context(error::SqliteSnafu)?;
    let mut insert_tag_stmt = conn.prepare("INSERT INTO tag (id, dateModified, primaryAliasId, description, categoryId) 
        VALUES (?, ?, (SELECT id FROM tag_alias WHERE name = ?), ?, (SELECT id FROM tag_category WHERE name = ?))").context(error::SqliteSnafu)?;
    let mut delete_tag_alias_stmt = conn.prepare("DELETE FROM tag_alias WHERE tagId = ?").context(error::SqliteSnafu)?;
    let mut delete_tag_stmt = conn.prepare("DELETE FROM tag WHERE id = ?").context(error::SqliteSnafu)?;

    // Insert new ones
    let mut insert_alias_stmt = conn.prepare("INSERT INTO tag_alias (tagId, name) VALUES (?, ?)").context(error::SqliteSnafu)?;
    for alias in changed_aliases {
        insert_alias_stmt.execute(params![alias.id, alias.value]).context(error::SqliteSnafu)?;
    }

    // Handle deleted tags
    let deleted_tag_ids = SqlVec(tags.iter().filter(|p| existing_ids.contains(&p.id) && p.deleted).map(|p| p.id).collect::<Vec<i64>>());
    // Remove from game tagsStr
    conn.execute("UPDATE game
    SET tagsStr = (
        SELECT IFNULL(string_agg(ta.name, '; '), '')
        FROM game_tags_tag gtt
        JOIN tag t ON gtt.tagId = t.id
        JOIN tag_alias ta ON t.primaryAliasId = ta.id
        WHERE gtt.gameId = game.id AND t.id NOT IN rarray(?)
    ) WHERE game.id IN (
        SELECT gameId FROM game_tags_tag WHERE tagId IN rarray(?) 
    )", params![deleted_tag_ids, deleted_tag_ids]).context(error::SqliteSnafu)?;
    // Remove all data
    conn.execute("DELETE FROM game_tags_tag WHERE tagId IN rarray(?)", params![deleted_tag_ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM tag_alias WHERE tagId IN rarray(?)", params![deleted_tag_ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM tag WHERE id IN rarray(?)", params![deleted_tag_ids]).context(error::SqliteSnafu)?;

    // Handle updated tags
    for tag in tags.iter().filter(|p| existing_ids.contains(&p.id) && !p.deleted) {
        update_tag_stmt.execute(params![tag.date_modified, tag.name, tag.description, tag.category, tag.id]).context(error::SqliteSnafu)?;
    }

    // Handle new tags
    for tag in tags.iter().filter(|p| !existing_ids.contains(&p.id) && !p.deleted) {
        // Clean up any 'loose' rows
        delete_tag_alias_stmt.execute(params![tag.id]).context(error::SqliteSnafu)?;
        delete_tag_stmt.execute(params![tag.id]).context(error::SqliteSnafu)?;

        // Insert new tag entry (above already added aliases)
        for alias in &tag.aliases {
            insert_alias_stmt.execute(params![tag.id, &alias]).context(error::SqliteSnafu)?;
        }
        insert_tag_stmt.execute(params![tag.id, tag.date_modified, tag.name, tag.description, tag.category]).context(error::SqliteSnafu)?;
    }

    mark_index_dirty(conn).context(error::SqliteSnafu)?;

    Ok(())
}

pub fn apply_games(conn: &Connection, games_res: &RemoteGamesRes) -> Result<()> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn).context(error::SqliteSnafu)?;

    let changed_ids = SqlVec(games_res.games.iter().map(|g| g.id.clone()).collect::<Vec<String>>());

    println!("Reassigning relations");

    // Clear game relations
    conn.execute("DELETE FROM game_tags_tag WHERE gameId IN rarray(?)", params![changed_ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM game_platforms_platform WHERE gameId IN rarray(?)", params![changed_ids]).context(error::SqliteSnafu)?;
    // Insert game relations
    let mut insert_tag_relation_stmt = conn.prepare("INSERT INTO game_tags_tag (gameId, tagId) 
    VALUES (?, ?)").context(error::SqliteSnafu)?;
    let mut insert_platform_relation_stmt = conn.prepare("INSERT INTO game_platforms_platform (gameId, platformId) 
    VALUES (?, ?)").context(error::SqliteSnafu)?;
    for ta in &games_res.tag_relations {
        insert_tag_relation_stmt.execute(params![ta[0], ta[1]]).context(error::SqliteSnafu)?;
    }
    for pa in &games_res.platform_relations {
        insert_platform_relation_stmt.execute(params![pa[0], pa[1]]).context(error::SqliteSnafu)?;
    }

    println!("Reassigning add apps");

    // Unassign all add apps
    conn.execute("DELETE FROM additional_app WHERE parentGameId IN rarray(?)", params![changed_ids]).context(error::SqliteSnafu)?;
    // Reassign all add apps
    let mut insert_add_app_stmt = conn.prepare("INSERT INTO additional_app
    (id, applicationPath, launchCommand, name, parentGameId, autoRunBefore, waitForExit)
    VALUES
    (?, ?, ?, ?, ?, ?, ?)").context(error::SqliteSnafu)?;
    for aa in &games_res.add_apps {
        insert_add_app_stmt.execute(params![Uuid::new_v4().to_string(), aa.application_path, aa.launch_command, aa.name, aa.parent_game_id,
            aa.auto_run_before, aa.wait_for_exit])
            .context(error::SqliteSnafu)?;
    }

    println!("Reassigning game data");

    // Unassign all removed game data (if it isn't already downloaded)
    conn.execute("DELETE FROM game_data WHERE gameId IN rarray(?) AND presentOnDisk == false", params![changed_ids]).context(error::SqliteSnafu)?;
    // Assign all new game data
    let mut insert_game_data_stmt = conn.prepare("INSERT INTO game_data
    (gameId, title, dateAdded, sha256, crc32, presentOnDisk, path, size, parameters, applicationPath, launchCommand)
    VALUES
    (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(gameId, dateAdded)
    DO UPDATE SET parameters = ?, applicationPath = ?, launchCommand = ?").context(error::SqliteSnafu)?;
    for gd in &games_res.game_data {
        insert_game_data_stmt.execute(params![gd.game_id, gd.title, gd.date_added, gd.sha_256,
            gd.crc_32, false, "", gd.size, gd.parameters, gd.application_path, gd.launch_command,
            gd.parameters, gd.application_path, gd.launch_command])
            .context(error::SqliteSnafu)?;
    }

    let existing_ids = game::find_all_ids(conn).context(error::SqliteSnafu)?;

    println!("Updating games");

    // Handle updated games
    let mut update_game_stmt = conn.prepare("UPDATE game SET library = ?, title = ?, alternateTitles = ?, series = ?, developer = ?, publisher = ?,
        platformName = ?, platformId = (SELECT platformId FROM platform_alias WHERE name = ?), platformsStr = ?, dateAdded = ?, dateModified = ?, 
        playMode = ?, status = ?, notes = ?, source = ?, activeDataId = -1,
        applicationPath = ?, launchCommand = ?, releaseDate = ?, version = ?,
        originalDescription = ?, language = ?, archiveState = ?, ruffleSupport = ? WHERE id = ?").context(error::SqliteSnafu)?;

    for g in games_res.games.iter().filter(|p| existing_ids.contains(&p.id)) {
        update_game_stmt.execute(params![
            g.library, g.title, g.alternate_titles, g.series, g.developer, g.publisher,
            g.platform_name, g.platform_name, "", g.date_added, g.date_modified,
            g.play_mode, g.status, g.notes, g.source,
            g.application_path, g.launch_command, g.release_date, g.version,
            g.original_description, g.language, g.archive_state, g.ruffle_support, g.id]).context(error::SqliteSnafu)?;
    }

    println!("Inserting games");

    // Handle new games
    let mut insert_game_stmt = conn.prepare("INSERT INTO game (id, library, title, alternateTitles, series, developer, publisher,
        platformName, platformId, platformsStr, dateAdded, dateModified, broken, extreme, playMode, status,
        notes, tagsStr, source, applicationPath, launchCommand, releaseDate, version,
        originalDescription, language, activeDataId, activeDataOnDisk, playtime,
        archiveState, orderTitle, ruffleSupport) VALUES (?, ?, ?, ?, ?, ?, ?,
        ?, ?, (SELECT platformId FROM platform_alias WHERE name = ?), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)").context(error::SqliteSnafu)?;

    for g in games_res.games.iter().filter(|p| !existing_ids.contains(&p.id)) {
        insert_game_stmt.execute(params![
            g.id, g.library, g.title, g.alternate_titles, g.series, g.developer, g.publisher,
            g.platform_name, g.platform_name, "", g.date_added, g.date_modified, false, false, g.play_mode, g.status,
            g.notes, "", g.source, g.application_path, g.launch_command, g.release_date, g.version,
            g.original_description, g.language, -1, false, 0,
            g.archive_state, "", g.ruffle_support,
        ]).context(error::SqliteSnafu)?;
    }

    println!("Updating games - cleanup");

    // Update platformStr and tagsStr for all changed games
    conn.execute("UPDATE game
    SET tagsStr = (
        SELECT IFNULL(string_agg(ta.name, '; '), '')
        FROM game_tags_tag gtt
        JOIN tag t ON gtt.tagId = t.id
        JOIN tag_alias ta ON t.primaryAliasId = ta.id
        WHERE gtt.gameId = game.id
    ) WHERE game.id IN rarray(?)", params![changed_ids]).context(error::SqliteSnafu)?;
    conn.execute("UPDATE game
    SET platformsStr = (
        SELECT IFNULL(string_agg(pa.name, '; '), '')
        FROM game_platforms_platform gpp
        JOIN platform p ON gpp.platformId = p.id
        JOIN platform_alias pa ON p.primaryAliasId = pa.id
        WHERE gpp.gameId = game.id
    ) WHERE game.id IN rarray(?)", params![changed_ids]).context(error::SqliteSnafu)?;

    println!("Active game id cleanup");

    // Update active game id info
    conn.execute("UPDATE game
    SET activeDataId = (SELECT game_data.id FROM game_data WHERE game.id = game_data.gameId ORDER BY game_data.dateAdded DESC LIMIT 1)
    WHERE game.activeDataId = -1", ()).context(error::SqliteSnafu)?;

    mark_index_dirty(conn).context(error::SqliteSnafu)?;

    Ok(())
}

pub fn delete_games(conn: &Connection, games_res: &RemoteDeletedGamesRes) -> Result<()> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn).context(error::SqliteSnafu)?;

    let ids = SqlVec(games_res.games.iter().map(|g| g.id.clone()).collect::<Vec<String>>());

    conn.execute("DELETE FROM game_tags_tag WHERE gameId IN rarray(?)", params![ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM game_platforms_platform WHERE gameId IN rarray(?)", params![ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM game_data WHERE gameId IN rarray(?)", params![ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM additional_app WHERE parentGameId IN rarray(?)", params![ids]).context(error::SqliteSnafu)?;
    conn.execute("DELETE FROM game WHERE id IN rarray(?)", params![ids]).context(error::SqliteSnafu)?;

    Ok(())
}

pub fn apply_redirects(conn: &Connection, redirects: Vec<GameRedirect>) -> Result<()> {
    let mut stmt = conn.prepare("INSERT OR IGNORE INTO game_redirect (sourceId, id) VALUES (?, ?)").context(error::SqliteSnafu)?;
    for r in redirects.iter() {
        stmt.execute(params![r.source_id, r.dest_id]).context(error::SqliteSnafu)?;
    }
    conn.execute("DELETE FROM game_redirect WHERE sourceId IN (SELECT id FROM game)", ()).context(error::SqliteSnafu)?;
    Ok(())
}
