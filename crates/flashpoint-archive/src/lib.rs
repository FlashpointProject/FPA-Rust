use std::collections::HashMap;
use game::{PartialGame, search::{GameSearch, PageTuple}, Game, AdditionalApp};
use game_data::{GameData, PartialGameData};
use platform::PlatformAppPath;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use snafu::ResultExt;
use tag::{PartialTag, Tag, TagSuggestion};
use tag_category::{TagCategory, PartialTagCategory};
use chrono::Utc;

mod error;
use error::{Error, Result};
use update::{RemoteCategory, RemoteDeletedGamesRes, RemoteGamesRes, RemotePlatform, RemoteTag};

pub mod game;
pub mod game_data;
mod migration;
pub mod platform;
pub mod tag;
pub mod tag_category;
pub mod update;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

pub struct FlashpointArchive {
    pool: Option<Pool<SqliteConnectionManager>>
}

impl FlashpointArchive {
    pub fn new() -> FlashpointArchive {
        FlashpointArchive {
            pool: None,
        }
    }

    /// Load a new database for Flashpoint. Open databases will close.
    /// 
    /// `source` - Path to database file, or :memory: to open a fresh database in memory
    pub fn load_database(&mut self, source: &str) -> Result<()> {
        let conn_manager = if source == ":memory:" {
            SqliteConnectionManager::memory()
        } else {
            SqliteConnectionManager::file(source)
        };

        let pool = r2d2::Pool::new(conn_manager).expect("Failed to open R2D2 conn pool");
        let mut conn = pool.get().unwrap();

        // Perform database migrations
        migration::up(&mut conn).context(error::DatabaseMigrationSnafu)?;
        conn.execute("PRAGMA foreign_keys=off;", ()).context(error::SqliteSnafu)?;
        // Always make there's always a default tag category present 
        tag_category::find_or_create(&conn, "default", None).context(error::SqliteSnafu)?;

        self.pool = Some(pool);

        Ok(())
    }

    pub async fn search_games(&self, search: &GameSearch) -> Result<Vec<game::Game>> {
        with_connection!(&self.pool, |conn| {
            game::search::search(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn search_games_index(&self, search: &mut GameSearch) -> Result<Vec<PageTuple>> {
        with_connection!(&self.pool, |conn| {
            game::search::search_index(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn search_games_total(&self, search: &GameSearch) -> Result<i64> {
        with_connection!(&self.pool, |conn| {
            game::search::search_count(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn search_games_with_tag(&self, tag: &str) -> Result<Vec<Game>> {
        with_connection!(&self.pool, |conn| {
            game::find_with_tag(conn, tag).context(error::SqliteSnafu)
        })
    }

    pub async fn search_games_random(&self, search: &GameSearch, count: i64) -> Result<Vec<Game>> {
        with_connection!(&self.pool, |conn| {
            game::search::search_random(conn, search.clone(), count).context(error::SqliteSnafu)
        })
    }

    pub async fn search_tag_suggestions(&self, partial: &str, blacklist: Vec<String>) -> Result<Vec<TagSuggestion>> {
        with_connection!(&self.pool, |conn| {
            tag::search_tag_suggestions(conn, partial, blacklist).context(error::SqliteSnafu)
        })
    }

    pub async fn search_platform_suggestions(&self, partial: &str) -> Result<Vec<TagSuggestion>> {
        with_connection!(&self.pool, |conn| {
            platform::search_platform_suggestions(conn, partial).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_ids(&self) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_all_ids(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_game(&self, id: &str) -> Result<Option<Game>> {
        with_connection!(&self.pool, |conn| {
            game::find(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn create_game(&self, partial_game: &PartialGame) -> Result<game::Game> {
        with_transaction!(&self.pool, |tx| {
            game::create(tx, partial_game).context(error::SqliteSnafu)
        })
    }

    pub async fn save_game(&self, partial_game: &mut PartialGame) -> Result<Game> {
        with_transaction!(&self.pool, |tx| {
            match partial_game.date_modified {
                Some(_) => (),
                None => partial_game.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            }
            game::save(tx, partial_game).context(error::SqliteSnafu)
        })
    }

    pub async fn save_games(&self, partial_games: Vec<&mut PartialGame>) -> Result<()> {
        with_transaction!(&self.pool, |tx| {
            for partial_game in partial_games {
                match partial_game.date_modified {
                    Some(_) => (),
                    None => partial_game.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
                }
                game::save(tx, partial_game).context(error::SqliteSnafu)?;
            }
            Ok(())
        })
    }

    pub async fn delete_game(&self, id: &str) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            game::delete(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn count_games(&self) -> Result<i64> {
        with_connection!(&self.pool, |conn| {
            game::count(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_add_app_by_id(&self, id: &str) -> Result<Option<AdditionalApp>> {
        with_connection!(&self.pool, |conn| {
            game::find_add_app_by_id(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn create_add_app(&self, add_app: &mut AdditionalApp) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            game::create_add_app(conn, add_app).context(error::SqliteSnafu)
        })
    }

    pub async fn find_game_data_by_id(&self, game_data_id: i64) -> Result<Option<GameData>> {
        with_connection!(&self.pool, |conn| {
            game::find_game_data_by_id(conn, game_data_id).context(error::SqliteSnafu)
        })
    }

    pub async fn find_game_data(&self, game_id: &str) -> Result<Vec<GameData>> {
        with_connection!(&self.pool, |conn| {
            game::get_game_data(conn, game_id).context(error::SqliteSnafu)
        })
    }

    pub async fn create_game_data(&self, game_data: &PartialGameData) -> Result<GameData> {
        with_connection!(&self.pool, |conn| {
            game::create_game_data(conn, game_data).context(error::SqliteSnafu)
        })
    }

    pub async fn save_game_data(&self, game_data: &PartialGameData) -> Result<GameData> {
        with_connection!(&self.pool, |conn| {
            game::save_game_data(conn, game_data).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_game_data(&self, id: i64) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            game_data::delete(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_tags(&self) -> Result<Vec<Tag>> {
        with_connection!(&self.pool, |conn| {
            tag::find(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_tag(&self, name: &str) -> Result<Option<Tag>> {
        with_connection!(&self.pool, |conn| {
            tag::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub async fn find_tag_by_id(&self, id: i64) -> Result<Option<Tag>> {
        with_connection!(&self.pool, |conn| {
            tag::find_by_id(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn create_tag(&self, name: &str, category: Option<String>, id: Option<i64>) -> Result<Tag> {
        with_transaction!(&self.pool, |conn| {
            tag::create(conn, name, category, id).context(error::SqliteSnafu)
        })
    }

    pub async fn save_tag(&self, partial: &mut PartialTag) -> Result<Tag> {
        with_transaction!(&self.pool, |conn| {
            match partial.date_modified {
                Some(_) => (),
                None => partial.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            }
            tag::save(conn, &partial).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_tag(&self, name: &str) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            tag::delete(conn, name).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_tag_by_id(&self, id: i64) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            tag::delete_by_id(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn count_tags(&self) -> Result<i64> {
        with_connection!(&self.pool, |conn| {
            tag::count(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn merge_tags(&self, name: &str, merged_into: &str) -> Result<Tag> {
        with_transaction!(&self.pool, |conn| {
            tag::merge_tag(conn, name, merged_into).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_platforms(&self) -> Result<Vec<Tag>> {
        with_connection!(&self.pool, |conn| {
            platform::find(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_platform(&self, name: &str) -> Result<Option<Tag>> {
        with_connection!(&self.pool, |conn| {
            platform::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub async fn find_platform_by_id(&self, id: i64) -> Result<Option<Tag>> {
        with_connection!(&self.pool, |conn| {
            platform::find_by_id(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn create_platform(&self, name: &str, id: Option<i64>) -> Result<Tag> {
        with_transaction!(&self.pool, |conn| {
            platform::create(conn, name, id).context(error::SqliteSnafu)
        })
    }

    pub async fn save_platform(&self, partial: &mut PartialTag) -> Result<Tag> {
        with_transaction!(&self.pool, |conn| {
            match partial.date_modified {
                Some(_) => (),
                None => partial.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            }
            platform::save(conn, &partial).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_platform(&self, name: &str) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            platform::delete(conn, name).context(error::SqliteSnafu)
        })
    }

    pub async fn count_platforms(&self) -> Result<i64> {
        with_connection!(&self.pool, |conn| {
            platform::count(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_tag_categories(&self) -> Result<Vec<TagCategory>> {
        with_connection!(&self.pool, |conn| {
            tag_category::find(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_tag_category(&self, name: &str) -> Result<Option<TagCategory>> {
        with_connection!(&self.pool, |conn| {
            tag_category::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub async fn find_tag_category_by_id(&self, id: i64) -> Result<Option<TagCategory>> {
        with_connection!(&self.pool, |conn| {
            tag_category::find_by_id(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn create_tag_category(&self, partial: &PartialTagCategory) -> Result<TagCategory> {
        with_connection!(&self.pool, |conn| {
            tag_category::create(conn, partial).context(error::SqliteSnafu)
        })
    }

    pub async fn save_tag_category(&self, partial: &PartialTagCategory) -> Result<TagCategory> {
        with_connection!(&self.pool, |conn| {
            tag_category::save(conn, partial).context(error::SqliteSnafu)
        })
    }

    pub async fn new_tag_filter_index(&self, search: &mut GameSearch) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            game::search::new_tag_filter_index(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_libraries(&self) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_libraries(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_statuses(&self) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_statuses(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_play_modes(&self) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_play_modes(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_application_paths(&self) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_application_paths(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn find_platform_app_paths(&self) -> Result<HashMap<String, Vec<PlatformAppPath>>> {
        with_connection!(&self.pool, |conn| {
            game::find_platform_app_paths(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn add_game_playtime(&self, game_id: &str, seconds: i64) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            game::add_playtime(conn, game_id, seconds).context(error::SqliteSnafu)
        })
    }

    pub async fn clear_playtime_tracking(&self) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            game::clear_playtime_tracking(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn force_games_active_data_most_recent(&self) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            game::force_active_data_most_recent(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn update_apply_categories(&self, cats: Vec<RemoteCategory>) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            update::apply_categories(conn, cats)
        })
    }

    pub async fn update_apply_platforms(&self, platforms: Vec<RemotePlatform>) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            update::apply_platforms(conn, platforms)
        })
    }
    
    pub async fn update_apply_tags(&self, tags: Vec<RemoteTag>) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            update::apply_tags(conn, tags)
        })
    }

    pub async fn update_apply_games(&self, games_res: &RemoteGamesRes) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            update::apply_games(conn, games_res)
        })
    }
    pub async fn update_delete_games(&self, games_res: &RemoteDeletedGamesRes) -> Result<()> {
        with_transaction!(&self.pool, |conn| {
            update::delete_games(conn, games_res)
        })
    }

    pub async fn optimize_database(&self) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            optimize_database(conn).context(error::SqliteSnafu)
        })
    }
}

fn optimize_database(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("ANALYZE", ())?;
    conn.execute("REINDEX", ())?;
    conn.execute("VACUUM", ())?;
    Ok(())
}

#[macro_export]
macro_rules! with_connection {
    ($pool:expr, $body:expr) => {
        match $pool {
            Some(conn) => {
                let conn = &conn.get().unwrap();
                conn.execute("PRAGMA foreign_keys=off;", ()).context(error::SqliteSnafu)?;
                $body(conn)
            },
            None => return Err(Error::DatabaseNotInitialized)
        }
    };
}

#[macro_export]
macro_rules! with_transaction {
    ($pool:expr, $body:expr) => {
        match $pool {
            Some(conn) => {
                let mut conn = conn.get().unwrap();
                conn.execute("PRAGMA foreign_keys=off;", ()).context(error::SqliteSnafu)?;
                let tx = conn.transaction().context(error::SqliteSnafu)?;
                let res = $body(&tx);
                if res.is_ok() {
                    println!("Applying transaction");
                    tx.commit().context(error::SqliteSnafu)?;
                    println!("Applied transaction");
                }
                res
            },
            None => return Err(Error::DatabaseNotInitialized)
        }
    };
}

#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => (if ::std::cfg!(debug_assertions) { ::std::println!($($arg)*); })
}

#[cfg(test)]
mod tests {

    use crate::game::search::{GameSearchOffset, GameFilter, FieldFilter};

    use super::*;

    const TEST_DATABASE: &str = "benches/flashpoint.sqlite";

    #[tokio::test]
    async fn database_not_initialized() {
        let flashpoint = FlashpointArchive::new();
        let result = flashpoint.count_games().await;
        assert!(result.is_err());

        let e = result.unwrap_err();
        assert!(matches!(e, Error::DatabaseNotInitialized {}));
    }

    #[tokio::test]
    async fn migrations_valid() {
        let migrations = migration::get();
        assert!(migrations.validate().is_ok());
    }

    #[tokio::test]
    async fn count_games() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let result = flashpoint.count_games().await;
        assert!(result.is_ok());

        let total = result.unwrap();
        assert_eq!(total, 191150);
    }

    #[tokio::test]
    async fn search_full_scan() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        search.limit = 99999999999;
        search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
        let result = flashpoint.search_games(&search).await;
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 162929);
    }

    #[tokio::test]
    async fn search_tags_or() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        search.limit = 99999999999;
        search.filter.match_any = true;
        search.filter.exact_whitelist.tags = Some(vec!["Action".to_owned(), "Adventure".to_owned()]);
        let result = flashpoint.search_games(&search).await;
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 36724);
    }

    #[tokio::test]
    async fn search_tags_and() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        search.limit = 99999999999;
        search.filter.match_any = false;
        search.filter.exact_whitelist.tags = Some(vec!["Action".to_owned(), "Adventure".to_owned()]);
        let result = flashpoint.search_games(&search).await;
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 397);
    }

    #[tokio::test]
    async fn search_tags_and_or_combined() {
        // Has 'Action' or 'Adventure', but is missing 'Sonic The Hedgehog'
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        let mut inner_filter = game::search::GameFilter::default();
        // Uncap limit, we want an accurate count
        search.limit = 30000;
        // Add the OR to an inner filter
        inner_filter.exact_whitelist.tags = Some(vec!["Action".to_owned(), "Adventure".to_owned()]);
        inner_filter.match_any = true; // OR
        // Add the AND to the main filter, with the inner filter
        search.filter.subfilters = vec![inner_filter];
        search.filter.exact_blacklist.tags = Some(vec!["Sonic The Hedgehog".to_owned()]);
        search.filter.match_any = false; // AND

        // Test total results
        let total_result = flashpoint.search_games_total(&search).await;
        assert!(total_result.is_ok());
        let total = total_result.unwrap();
        assert_eq!(total, 36541);

        // Test first page results
        let result = flashpoint.search_games(&search).await;
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 30000);
        let page_end_game = games.last().unwrap();

        // Test index
        let index_result = flashpoint.search_games_index(&mut search).await;
        assert!(index_result.is_ok());
        let index = index_result.unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].id, page_end_game.id);

        // Test last page results
        search.offset = Some(GameSearchOffset{
            value: page_end_game.title.clone(),
            game_id: page_end_game.id.clone(),
            title: page_end_game.title.clone(),
        });
        let last_result = flashpoint.search_games(&search).await;
        assert!(last_result.is_ok());
        let last_page = last_result.unwrap();
        assert_eq!(last_page.len(), 6541);
    }

    #[tokio::test]
    async fn search_multiple_subfilters() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = GameSearch::default();
        search.filter.subfilters.push(GameFilter {
            exact_blacklist: FieldFilter {
                tags: Some(vec!["Action".to_owned(), "Shooting".to_owned()]),
                ..Default::default()
            },
            ..Default::default()
        });
        search.filter.subfilters.push(GameFilter {
            exact_blacklist: FieldFilter {
                tags: Some(vec!["Adventure".to_owned()]),
                ..Default::default()
            },
            ..Default::default()
        });
        search.filter.exact_whitelist.library = Some(vec!["arcade".to_owned()]);
        search.filter.match_any = false;
        assert!(flashpoint.search_games_index(&mut search).await.is_ok());
    }

    #[tokio::test]
    async fn parse_user_search_input_assorted() {
        game::search::parse_user_input("test");
        game::search::parse_user_input(r#"tag:"sonic""#);
        game::search::parse_user_input(r#"o_%$ dev:"san" disk t:7 potato"#);
    }

    #[tokio::test]
    async fn parse_user_search_input_sizes() {
        let search = game::search::parse_user_input("tags>5 addapps=3 gamedata<12 test>generic");
        assert!(search.filter.higher_than.tags.is_some());
        assert_eq!(search.filter.higher_than.tags.unwrap(), 5);
        assert!(search.filter.equal_to.add_apps.is_some());
        assert_eq!(search.filter.equal_to.add_apps.unwrap(), 3);
        assert!(search.filter.lower_than.game_data.is_some());
        assert_eq!(search.filter.lower_than.game_data.unwrap(), 12);
        assert!(search.filter.whitelist.generic.is_some());
        let generics = search.filter.whitelist.generic.unwrap();
        assert_eq!(generics.len(), 1);
        assert_eq!(generics[0], "test>generic");
    }

    #[tokio::test]
    async fn find_game() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let result = flashpoint.find_game("00deff25-5cd2-40d1-a0e7-151d82ce16c5").await;
        assert!(result.is_ok());
        let game_opt = result.unwrap();
        assert!(game_opt.is_some());
        let game = game_opt.unwrap();
        assert_eq!(game.title, "Crab Planet");
        assert!(game.detailed_platforms.is_some());
        let platforms = game.detailed_platforms.unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].name, "Flash");
    }

    #[tokio::test]
    async fn tag_categories() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let partial_tc = tag_category::PartialTagCategory {
            id: -1,
            name: "test".to_owned(),
            color: "#FF00FF".to_owned(),
            description: Some("test".to_owned()),
        };
        assert!(flashpoint.create_tag_category(&partial_tc).await.is_ok());
        let saved_cat_result = flashpoint.find_tag_category("test").await;
        assert!(saved_cat_result.is_ok());
        let saved_cat_opt = saved_cat_result.unwrap();
        assert!(saved_cat_opt.is_some());
        let saved_cat = saved_cat_opt.unwrap();
        assert_eq!(saved_cat.name, "test");
        assert_eq!(saved_cat.color, "#FF00FF");
        assert!(saved_cat.description.is_some());
        assert_eq!(saved_cat.description.unwrap(), "test");

        let all_cats_result = flashpoint.find_all_tag_categories().await;
        assert!(all_cats_result.is_ok());
        let all_cats = all_cats_result.unwrap();
        // Default category always exists
        assert_eq!(all_cats.len(), 2);
    }

    #[tokio::test]
    async fn create_and_save_game() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let partial_game = game::PartialGame {
            title: Some(String::from("Test Game")),
            tags: Some(vec!["Action"].into()),
            ..game::PartialGame::default()
        };
        let result = flashpoint.create_game(&partial_game).await;
        assert!(result.is_ok());
        let mut game = result.unwrap();
        let found_tag_res = flashpoint.find_tag("Action").await;
        assert!(found_tag_res.is_ok());
        let found_tag_opt = found_tag_res.unwrap();
        assert!(found_tag_opt.is_some());
        let found_game_res = flashpoint.find_game(&game.id).await;
        assert!(found_game_res.is_ok());
        let found_game_opt = found_game_res.unwrap();
        assert!(found_game_opt.is_some());
        let found_game = found_game_opt.unwrap();
        assert!(found_game.detailed_tags.is_some());
        let found_tags = found_game.detailed_tags.unwrap();
        assert_eq!(found_tags.len(), 1);
        assert_eq!(game.title, "Test Game");
        game.developer = String::from("Newgrounds");
        game.tags = vec!["Action", "Adventure"].into();
        game.primary_platform = String::from("Flash");
        let save_result = flashpoint.save_game(&mut game.into()).await;
        assert!(save_result.is_ok());
        let saved_game = save_result.unwrap();
        assert_eq!(saved_game.developer, "Newgrounds");
        assert_eq!(saved_game.tags.len(), 2);
        assert_eq!(saved_game.platforms.len(), 1);
        assert_eq!(saved_game.platforms[0], "Flash");
        assert_eq!(saved_game.primary_platform, "Flash");
        assert!(saved_game.detailed_platforms.is_some());
        let detailed_platforms = saved_game.detailed_platforms.unwrap();
        assert_eq!(detailed_platforms.len(), 1);
        assert!(saved_game.detailed_tags.is_some());
        let detailed_tags = saved_game.detailed_tags.unwrap();
        assert_eq!(detailed_tags.len(), 2);
        assert_eq!(detailed_tags[0].name, "Action");
    }

    #[tokio::test]
    async fn parse_user_search_input() {
        let input = r#"sonic title:"dog cat" -title:"cat dog" tag:Action -mario"#;
        let search = game::search::parse_user_input(input);
        println!("{:?}", search);
        assert!(search.filter.whitelist.generic.is_some());
        assert_eq!(search.filter.whitelist.generic.unwrap()[0], "sonic");
        assert!(search.filter.whitelist.title.is_some());
        assert_eq!(search.filter.whitelist.title.unwrap()[0], "dog cat");
        assert!(search.filter.blacklist.title.is_some());
        assert_eq!(search.filter.blacklist.title.unwrap()[0], "cat dog");
        assert!(search.filter.whitelist.tags.is_some());
        assert_eq!(search.filter.whitelist.tags.unwrap()[0], "Action");
        assert!(search.filter.blacklist.generic.is_some());
        assert_eq!(search.filter.blacklist.generic.unwrap()[0], "mario");
    }

    #[tokio::test]
    async fn parse_user_quick_search_input() {
        let input = r#"#Action -!Flash @"armor games" !"#;
        let search = game::search::parse_user_input(input);
        assert!(search.filter.whitelist.tags.is_some());
        assert_eq!(search.filter.whitelist.tags.unwrap()[0], "Action");
        assert!(search.filter.blacklist.platforms.is_some());
        assert_eq!(search.filter.blacklist.platforms.unwrap()[0], "Flash");
        assert!(search.filter.whitelist.developer.is_some());
        assert_eq!(search.filter.whitelist.developer.unwrap()[0], "armor games");
        assert!(search.filter.whitelist.generic.is_some());
        assert_eq!(search.filter.whitelist.generic.unwrap()[0], "!");
    }

    #[tokio::test]
    async fn parse_user_exact_search_input() {
        let input = r#"=!Flash -=publisher:Newgrounds =sonic"#;
        let search = game::search::parse_user_input(input);
        assert!(search.filter.exact_whitelist.platforms.is_some());
        assert_eq!(search.filter.exact_whitelist.platforms.unwrap()[0], "Flash");
        assert!(search.filter.exact_blacklist.publisher.is_some());
        assert_eq!(search.filter.exact_blacklist.publisher.unwrap()[0], "Newgrounds");
        assert!(search.filter.whitelist.generic.is_some());
        assert!(search.filter.exact_whitelist.generic.is_none());
        assert_eq!(search.filter.whitelist.generic.unwrap()[0], "=sonic");
    }

    #[tokio::test]
    async fn find_all_game_libraries() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let libraries_res = flashpoint.find_all_game_libraries().await;
        assert!(libraries_res.is_ok());
        let libraries = libraries_res.unwrap();
        assert_eq!(libraries.len(), 2);
    }

    #[tokio::test]
    async fn create_tag() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let new_tag_res = flashpoint.create_tag("test", None, None).await;
        assert!(new_tag_res.is_ok());
        let new_tag = new_tag_res.unwrap();
        assert!(new_tag.category.is_some());
        assert_eq!(new_tag.category.unwrap(), "default");
        assert_eq!(new_tag.name, "test");
        assert_eq!(new_tag.aliases.len(), 1);
        assert_eq!(new_tag.aliases[0], "test");
    }

    #[tokio::test]
    async fn delete_tag() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let partial = PartialGame {
            title: Some("test".to_owned()),
            tags: Some(vec!["Action"].into()),
            ..Default::default()
        };
        let new_game_res = flashpoint.create_game(&partial).await;
        assert!(new_game_res.is_ok());
        let saved_game = new_game_res.unwrap();
        assert_eq!(saved_game.tags.len(), 1);
        let delete_res = flashpoint.delete_tag("Action").await;
        assert!(delete_res.is_ok());
        let modded_game_res = flashpoint.find_game(&saved_game.id).await;
        assert!(modded_game_res.is_ok());
        let modded_game_opt = modded_game_res.unwrap();
        assert!(modded_game_opt.is_some());
        let modded_game = modded_game_opt.unwrap();
        assert_eq!(modded_game.tags.len(), 0);
    }

    #[tokio::test]
    async fn merge_tags() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let partial = PartialGame {
            title: Some("test".to_owned()),
            tags: Some(vec!["Action"].into()),
            ..Default::default()
        };
        let new_game_res = flashpoint.create_game(&partial).await;
        assert!(new_game_res.is_ok());
        assert!(flashpoint.create_tag("Adventure", None, None).await.is_ok());
        let saved_game = new_game_res.unwrap();
        let merged_tag_res = flashpoint.merge_tags("Action", "Adventure").await;
        assert!(merged_tag_res.is_ok());
        let merged_tag = merged_tag_res.unwrap();
        assert_eq!(merged_tag.aliases.len(), 2);
        let modded_game_res = flashpoint.find_game(&saved_game.id).await;
        assert!(modded_game_res.is_ok());
        let modded_game_opt = modded_game_res.unwrap();
        assert!(modded_game_opt.is_some());
        let modded_game = modded_game_opt.unwrap();
        assert_eq!(modded_game.tags.len(), 1);
        assert_eq!(modded_game.tags[0], "Adventure");
    }

    #[tokio::test]
    async fn delete_platform() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let partial = PartialGame {
            title: Some("test".to_owned()),
            platforms: Some(vec!["Flash"].into()),
            ..Default::default()
        };
        let new_game_res = flashpoint.create_game(&partial).await;
        assert!(new_game_res.is_ok());
        let saved_game = new_game_res.unwrap();
        assert_eq!(saved_game.platforms.len(), 1);
        let delete_res = flashpoint.delete_platform("Flash").await;
        assert!(delete_res.is_ok());
        let modded_game_res = flashpoint.find_game(&saved_game.id).await;
        assert!(modded_game_res.is_ok());
        let modded_game_opt = modded_game_res.unwrap();
        assert!(modded_game_opt.is_some());
        let modded_game = modded_game_opt.unwrap();
        assert_eq!(modded_game.platforms.len(), 0);
    }

    #[tokio::test]
    async fn create_platform() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let new_tag_res = flashpoint.create_platform("test", None).await;
        assert!(new_tag_res.is_ok());
        let new_tag = new_tag_res.unwrap();
        assert!(new_tag.category.is_none());
        assert_eq!(new_tag.name, "test");
        assert_eq!(new_tag.aliases.len(), 1);
        assert_eq!(new_tag.aliases[0], "test");
    }

    #[tokio::test]
    async fn search_tag_suggestions() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let new_tag_res = flashpoint.create_tag("Action", None, None).await;
        assert!(new_tag_res.is_ok());
        let suggs_res = flashpoint.search_tag_suggestions("Act", vec![]).await;
        assert!(suggs_res.is_ok());
        assert_eq!(suggs_res.unwrap().len(), 1);
        let suggs_bad_res = flashpoint.search_tag_suggestions("Adventure", vec![]).await;
        assert!(suggs_bad_res.is_ok());
        assert_eq!(suggs_bad_res.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn update_game_when_platform_changed() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let partial_game = game::PartialGame {
            title: Some(String::from("Test Game")),
            tags: Some(vec!["Action"].into()),
            platforms: Some(vec!["Flash", "HTML5"].into()),
            primary_platform: Some("HTML5".into()),
            ..game::PartialGame::default()
        };
        let result = flashpoint.create_game(&partial_game).await;
        assert!(result.is_ok());
        let old_game = result.unwrap();
        let mut platform = flashpoint.find_platform("HTML5").await.unwrap().unwrap();
        println!("{} - {}", platform.id, platform.name);
        platform.name = String::from("Wiggle");
        let mut partial = PartialTag::from(platform);
        println!("{} - {}", partial.id, partial.name);
        let save_res = flashpoint.save_platform(&mut partial).await;
        assert!(save_res.is_ok());
        assert_eq!(save_res.unwrap().name, "Wiggle");
        let new_game = flashpoint.find_game(&old_game.id).await.unwrap().unwrap();
        println!("{}", new_game.platforms);
        assert_eq!(new_game.primary_platform, "Wiggle");
        assert!(new_game.platforms.contains(&"Wiggle".to_string()));
    }
}
