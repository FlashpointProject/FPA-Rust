use chrono::NaiveDateTime;
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, ValueRef, Value},
    Connection, OptionalExtension, Result,
};
use uuid::Uuid;
use std::{ops::{Deref, DerefMut}, vec::Vec, rc::Rc};

use crate::{tag::{Tag, self}, platform, game_data::GameData};

#[derive(Debug, Clone)]
pub struct TagVec(Vec<String>);

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
        TagVec(vec![])
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
        TagVec(strings)
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
    pub date_added: NaiveDateTime,
    pub date_modified: NaiveDateTime,
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
    pub last_played: Option<NaiveDateTime>,
    pub playtime: i64,
    pub active_game_config_id: Option<i64>,
    pub active_game_config_owner: Option<String>,
    pub archive_state: i64,
    pub game_data: Option<Vec<GameData>>,
}

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
    pub date_added: Option<NaiveDateTime>,
    pub date_modified: Option<NaiveDateTime>,
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
    pub last_played: Option<NaiveDateTime>,
    pub playtime: Option<i64>,
    pub active_game_config_id: Option<i64>,
    pub active_game_config_owner: Option<String>,
    pub archive_state: Option<i64>,
}

pub fn find(conn: &Connection, id: &str) -> Result<Option<Game>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, alternateTitles, series, developer, publisher, platformsStr, \
        platformName, dateAdded, dateModified, broken, extreme, playMode, status, notes, \
        tagsStr, source, applicationPath, launchCommand, releaseDate, version, \
        originalDescription, language, activeDataId, activeDataOnDisk, lastPlayed, playtime, \
        activeGameConfigId, activeGameConfigOwner, archiveState, library \
        FROM game WHERE id = ?1",
    )?;

    let game_result = stmt
        .query_row(params![id], |row| {
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
                detailed_platforms: None,
                detailed_tags: None,
                game_data: None,
            })
        })
        .optional(); // Converts rusqlite::Error::QueryReturnedNoRows to None

    if let Ok(Some(mut game)) = game_result {
        game.detailed_platforms = Some(get_game_platforms(conn, id)?);
        game.detailed_tags = Some(get_game_tags(conn, id)?);
        game.game_data = Some(get_game_data(conn, id)?);
        Ok(Some(game))
    } else {
        Ok(None)
    }
}

pub fn create(conn: &Connection, partial: &PartialGame) -> Result<Game> {
    let mut game: Game = partial.into();

    let tags_copy = game.tags.clone();
    let platforms_copy = game.platforms.clone();
    game.tags = vec![].into();
    game.platforms = vec![].into();

    for name in tags_copy {
        let detailed_tag = tag::find_or_create(conn, &name)?;
        game.tags.push(detailed_tag.name);
    }

    for name in platforms_copy {
        let detailed_platform = platform::find_or_create(conn, &name)?;
        game.platforms.push(detailed_platform.name);
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

    Ok(game)
}

pub fn save(conn: &Connection, game: &PartialGame) -> Result<Game> {
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
            let detailed_platform = platform::find_or_create(conn, &name)?;
            detailed_platforms_copy.push(detailed_platform.clone());
            existing_game.platforms.push(detailed_platform.name);
        }

        // Update relations in database
        let tag_ids: Vec<i64> = detailed_tags_copy.iter().map(|t| t.id).collect::<Vec<i64>>();
        let tag_values = Rc::new(tag_ids.iter().copied().map(Value::from).collect::<Vec<Value>>());
        let mut stmt = conn.prepare("DELETE FROM game_tags_tag WHERE gameId = ?1 AND tagId NOT IN rarray(?2)")?;
        stmt.execute(params![existing_game.id.as_str(), tag_values]).map(|changes| changes as usize)?;
        for tag_id in tag_ids {
            stmt = conn.prepare("INSERT OR IGNORE INTO game_tags_tag (gameId, tagId) VALUES (?1, ?2)")?;
            stmt.execute(params![existing_game.id.as_str(), tag_id])?;
        }

        let platform_ids: Vec<i64> = detailed_platforms_copy.iter().map(|t| t.id).collect::<Vec<i64>>();
        let platform_values = Rc::new(platform_ids.iter().copied().map(Value::from).collect::<Vec<Value>>());
        let mut stmt = conn.prepare("DELETE FROM game_platforms_platform WHERE gameId = ?1 AND platformId NOT IN rarray(?2)")?;
        stmt.execute(params![existing_game.id.as_str(), platform_values]).map(|changes| changes as usize)?;
        for platform_id in platform_ids {
            stmt = conn.prepare("INSERT OR IGNORE INTO game_platforms_platform (gameId, platformId) VALUES (?1, ?2)")?;
            stmt.execute(params![existing_game.id.as_str(), platform_id])?;
        }


        // Write back the changes to the database
        conn.execute(
            "UPDATE game SET library = ?, title = ?, alternateTitles = ?, series = ?, developer = ?, publisher = ?, \
             platformName = ?, platformsStr = ?, dateAdded = ?, dateModified = ?, broken = ?, \
             extreme = ?, playMode = ?, status = ?, notes = ?, tagsStr = ?, source = ?, \
             applicationPath = ?, launchCommand = ?, releaseDate = ?, version = ?, \
             originalDescription = ?, language = ?, activeDataId = ?, activeDataOnDisk = ?, \
             lastPlayed = ?, playtime = ?, activeGameConfigId = ?, activeGameConfigOwner = ?, \
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
                &existing_game.active_game_config_id,
                &existing_game.active_game_config_owner,
                &existing_game.archive_state,
                &existing_game.id,
            ],
        )?;

        existing_game.detailed_platforms = get_game_platforms(conn, existing_game.id.as_str())?.into();
        existing_game.detailed_tags = get_game_tags(conn, existing_game.id.as_str())?.into();

        Ok(existing_game)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn delete(conn: &Connection, id: &str) -> Result<usize> {
    let mut stmt = conn.prepare("DELETE FROM game WHERE id = ?1")?;

    stmt.execute(params![id])
}

pub fn count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM game", (), |row| row.get::<_, i64>(0))
}

fn get_game_platforms(conn: &Connection, id: &str) -> Result<Vec<Tag>> {
    let mut platform_stmt = conn.prepare(
        "SELECT p.id, p.description, pa.name, p.dateModified FROM platform p
         INNER JOIN game_platforms_platform gpp ON gpp.platformId = p.id
         INNER JOIN platform_alias pa ON p.primaryAliasId = pa.id
         WHERE gpp.gameId = ?1",
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
            conn.prepare("SELECT pa.name FROM platform_alias pa WHERE pa.platformId = ?1")?;

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
         WHERE gtt.gameId = ?1",
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
            conn.prepare("SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?1")?;

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

fn get_game_data(conn: &Connection, id: &str) -> Result<Vec<GameData>> {
    let mut game_data: Vec<GameData> = vec![];

    let mut game_data_stmt = conn.prepare("
        SELECT id, title, dateAdded, sha256, crc32, presentOnDisk,
        path, size, parameters, applicationPath, launchCommand
        FROM game_data
        WHERE gameId = ?1
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
            date_added: NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
            date_modified: NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
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
            active_game_config_id: None,
            active_game_config_owner: None,
            archive_state: 0,
            game_data: None,
        }
    }
}

impl Game {
    fn apply_partial(&mut self, source: &PartialGame) {
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
    
        if let Some(date_added) = source.date_added {
            self.date_added = date_added;
        }
    
        if let Some(date_modified) = source.date_modified {
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
    
        if let Some(last_played) = source.last_played {
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
            id: game.id.clone(),
            library: Some(game.id.clone()),
            title: Some(game.title.clone()),
            alternate_titles: Some(game.alternate_titles.clone()),
            series: Some(game.series.clone()),
            developer: Some(game.developer.clone()),
            publisher: Some(game.publisher.clone()),
            primary_platform: Some(game.primary_platform.clone()),
            platforms: Some(new_plats),
            date_added: Some(game.date_added),
            date_modified: Some(game.date_modified),
            legacy_broken: Some(game.legacy_broken),
            legacy_extreme: Some(game.legacy_extreme),
            play_mode: Some(game.play_mode.clone()),
            status: Some(game.status.clone()),
            notes: Some(game.notes.clone()),
            tags: Some(game.tags.clone()),
            source: Some(game.source.clone()),
            legacy_application_path: Some(game.legacy_application_path.clone()),
            legacy_launch_command: Some(game.legacy_launch_command.clone()),
            release_date: Some(game.release_date.clone()),
            version: Some(game.version.clone()),
            original_description: Some(game.original_description.clone()),
            language: Some(game.language.clone()),
            active_data_id: game.active_data_id,
            active_data_on_disk: Some(game.active_data_on_disk),
            last_played: game.last_played,
            playtime: Some(game.playtime),
            active_game_config_id: game.active_game_config_id,
            active_game_config_owner: game.active_game_config_owner.clone(),
            archive_state: Some(game.archive_state),
        }
    }
}