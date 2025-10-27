use std::{collections::HashMap, sync::{Arc, Mutex, atomic::AtomicBool, mpsc}};
use game::{ext::ExtensionInfo, search::{GameFilter, GameSearch, PageTuple, ParsedInput}, AdditionalApp, Game, GameRedirect, PartialGame};
use game_data::{GameData, PartialGameData};
use platform::PlatformAppPath;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use snafu::ResultExt;
use tag::{PartialTag, Tag, TagSuggestion};
use tag_category::{TagCategory, PartialTagCategory};
use chrono::Utc;
use lazy_static::lazy_static;
use crate::logger::EventManager;

mod error;
use error::{Error, Result};
use update::{RemoteCategory, RemoteDeletedGamesRes, RemoteGamesRes, RemotePlatform, RemoteTag};
use util::ContentTreeNode;

pub mod game;
pub mod game_data;
mod migration;
pub mod platform;
pub mod tag;
pub mod tag_category;
pub mod update;
pub mod util;
mod logger;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub const MAX_SEARCH: i64 = 99999999999;

lazy_static! {
    static ref LOGGER: Arc<EventManager> = EventManager::new();
}

pub struct FlashpointArchive {
    pool: Option<Pool<SqliteConnectionManager>>,
    extensions: game::ext::ExtensionRegistry,
    write_mutex: Mutex<()>,
}

impl FlashpointArchive {
    pub fn new() -> Self {
        FlashpointArchive {
            pool: None,
            extensions: game::ext::ExtensionRegistry::new(),
            write_mutex: Mutex::new(()),
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

    pub fn parse_user_input(&self, input: &str) -> ParsedInput {
        game::search::parse_user_input(input, Some(&self.extensions.searchables))
    }

    pub fn register_extension(&mut self, ext: ExtensionInfo) -> Result<()> {
        with_serialized_transaction!(&self, |tx| {
            self.extensions.create_ext_indices(tx, ext.clone())
        })?;

        self.extensions.register_ext(ext);

        Ok(())
    }

    pub async fn search_games(&self, search: &GameSearch) -> Result<Vec<game::Game>> {
        with_connection!(&self.pool, |conn| {
            debug_println!("Getting search page");
            game::search::search(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn search_games_index(&self, search: &mut GameSearch, limit: Option<i64>) -> Result<Vec<PageTuple>> {
        with_connection!(&self.pool, |conn| {
            debug_println!("Getting search index");
            game::search::search_index(conn, search, limit).context(error::SqliteSnafu)
        })
    }

    pub async fn search_games_total(&self, search: &GameSearch) -> Result<i64> {
        with_connection!(&self.pool, |conn| {
            debug_println!("Getting search total");
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
        with_serialized_transaction!(&self, |tx| {
            game::create(tx, partial_game).context(error::SqliteSnafu)
        })
    }

    pub async fn save_game(&self, partial_game: &mut PartialGame) -> Result<Game> {
        with_serialized_transaction!(&self, |tx| {
            match partial_game.date_modified {
                Some(_) => (),
                None => partial_game.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            }
            game::save(tx, partial_game).context(error::SqliteSnafu)
        })
    }

    pub async fn save_games(&self, partial_games: Vec<&mut PartialGame>) -> Result<()> {
        with_serialized_transaction!(&self, |tx| {
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
        with_serialized_transaction!(&self, |conn| {
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
        with_serialized_transaction!(&self, |conn| {
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
        with_serialized_transaction!(&self, |conn| {
            tag::create(conn, name, category, id).context(error::SqliteSnafu)
        })
    }

    pub async fn save_tag(&self, partial: &mut PartialTag) -> Result<Tag> {
        with_serialized_transaction!(&self, |conn| {
            match partial.date_modified {
                Some(_) => (),
                None => partial.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            }
            tag::save(conn, &partial).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_tag(&self, name: &str) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            tag::delete(conn, name).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_tag_by_id(&self, id: i64) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            tag::delete_by_id(conn, id).context(error::SqliteSnafu)
        })
    }

    pub async fn count_tags(&self) -> Result<i64> {
        with_connection!(&self.pool, |conn| {
            tag::count(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn merge_tags(&self, name: &str, merged_into: &str) -> Result<Tag> {
        with_serialized_transaction!(&self, |conn| {
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
        with_serialized_transaction!(&self, |conn| {
            platform::create(conn, name, id).context(error::SqliteSnafu)
        })
    }

    pub async fn save_platform(&self, partial: &mut PartialTag) -> Result<Tag> {
        with_serialized_transaction!(&self, |conn| {
            match partial.date_modified {
                Some(_) => (),
                None => partial.date_modified = Some(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            }
            platform::save(conn, &partial).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_platform(&self, name: &str) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
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

    pub async fn find_all_game_developers(&self, search: Option<GameSearch>) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_developers(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_publishers(&self, search: Option<GameSearch>) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_publishers(conn, search).context(error::SqliteSnafu)
        })
    }

    pub async fn find_all_game_series(&self, search: Option<GameSearch>) -> Result<Vec<String>> {
        with_connection!(&self.pool, |conn| {
            game::find_series(conn, search).context(error::SqliteSnafu)
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
        with_serialized_transaction!(&self, |conn| {
            game::add_playtime(conn, game_id, seconds).context(error::SqliteSnafu)
        })
    }

    pub async fn clear_playtime_tracking_by_id(&self, game_id: &str) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            game::clear_playtime_tracking_by_id(conn, game_id).context(error::SqliteSnafu)
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

    pub async fn find_game_redirects(&self) -> Result<Vec<GameRedirect>> {
        with_connection!(&self.pool, |conn| {
            game::find_redirects(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn create_game_redirect(&self, src_id: &str, dest_id: &str) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            game::create_redirect(conn, src_id, dest_id).context(error::SqliteSnafu)
        })
    }

    pub async fn delete_game_redirect(&self, src_id: &str, dest_id: &str) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            game::delete_redirect(conn, src_id, dest_id).context(error::SqliteSnafu)
        })
    }

    pub async fn update_apply_categories(&self, cats: Vec<RemoteCategory>) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            update::apply_categories(conn, cats)
        })
    }

    pub async fn update_apply_platforms(&self, platforms: Vec<RemotePlatform>) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            update::apply_platforms(conn, platforms)
        })
    }
    
    pub async fn update_apply_tags(&self, tags: Vec<RemoteTag>) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            update::apply_tags(conn, tags)
        })
    }

    pub async fn update_apply_games(&self, games_res: &RemoteGamesRes, owner: &str) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            update::apply_games(conn, games_res, owner)
        })
    }

    pub async fn update_delete_games(&self, games_res: &RemoteDeletedGamesRes) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            update::delete_games(conn, games_res)
        })
    }

    pub async fn update_apply_redirects(&self, redirects_res: Vec<GameRedirect>) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            update::apply_redirects(conn, redirects_res)
        })
    }

    pub async fn optimize_database(&self) -> Result<()> {
        with_connection!(&self.pool, |conn| {
            optimize_database(conn).context(error::SqliteSnafu)
        })
    }

    pub async fn new_custom_id_order(&self, custom_id_order: Vec<String>) -> Result<()> {
        with_serialized_transaction!(&self, |conn| {
            game::search::new_custom_id_order(conn, custom_id_order).context(error::SqliteSnafu)
        })
    }
}

pub fn logger_subscribe() -> (crate::logger::SubscriptionId, mpsc::Receiver<crate::logger::LogEvent>) {
    LOGGER.subscribe()
}

pub fn logger_unsubscribe(id: crate::logger::SubscriptionId) {
    LOGGER.unsubscribe(id)
}

fn optimize_database(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("ANALYZE", ())?;
    conn.execute("REINDEX", ())?;
    conn.execute("VACUUM", ())?;
    Ok(())
}

pub fn generate_content_tree(root: &str) -> Result<ContentTreeNode> {
    util::gen_content_tree(root).map_err(|_| snafu::NoneError).context(error::ContentTreeSnafu)
}

pub fn copy_folder(src: &str, dest: &str) -> Result<u64> {
    util::copy_folder(src, dest).map_err(|_| snafu::NoneError).context(error::CopyFolderSnafu)
}

pub fn merge_game_filters(a: &GameFilter, b: &GameFilter) -> GameFilter {
    let mut new_filter = GameFilter::default();
    new_filter.subfilters = vec![a.clone(), b.clone()];

    if a.match_any && b.match_any {
        new_filter.match_any = true;
    }

    return new_filter;
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
                    tx.commit().context(error::SqliteSnafu)?;
                    debug_println!("Applied transaction");
                }
                res
            },
            None => return Err(Error::DatabaseNotInitialized)
        }
    };
}

#[macro_export]
macro_rules! with_serialized_transaction {
    ($archive:expr, $body:expr) => {
        {
            let _write_guard = $archive.write_mutex.lock().unwrap();
            with_transaction!($archive.pool.as_ref(), $body)
        }
    };
}

pub fn enable_debug() {
    DEBUG_ENABLED.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn disable_debug() {
    DEBUG_ENABLED.store(false, std::sync::atomic::Ordering::SeqCst);
}

pub fn debug_enabled() -> bool {
    DEBUG_ENABLED.load(std::sync::atomic::Ordering::SeqCst)
}

#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => (if $crate::debug_enabled() {
        ::std::println!($($arg)*);
        let formatted_message = ::std::format!($($arg)*);
        $crate::LOGGER.dispatch_event(formatted_message);
    })
}

#[cfg(test)]
mod tests {

    use crate::game::{ext::ExtSearchable, search::{parse_user_input, FieldFilter, GameFilter, GameSearchOffset, GameSearchSortable}};

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
        search.limit = MAX_SEARCH;
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
        search.limit = MAX_SEARCH;
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
        search.limit = MAX_SEARCH;
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
        // Set page size for index search
        search.limit = 30000;
        // Add the OR to an inner filter
        inner_filter.exact_whitelist.tags = Some(vec!["Action".to_owned(), "Adventure".to_owned()]);
        inner_filter.match_any = true; // OR
        // Add the AND to the main filter, with the inner filter
        search.filter.subfilters = vec![inner_filter];
        search.filter.exact_blacklist.tags = Some(vec!["Sonic The Hedgehog".to_owned()]);
        search.filter.match_any = false; // AND
        search.order.column = GameSearchSortable::TITLE;

        // Test total results
        enable_debug();
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
        let index_result = flashpoint.search_games_index(&mut search, None).await;
        assert!(index_result.is_ok());
        let index = index_result.unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index[0].id, page_end_game.id);

        // Test last page results
        search.offset = Some(GameSearchOffset{
            value: serde_json::Value::String(page_end_game.title.clone()),
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
        assert!(flashpoint.search_games_index(&mut search, None).await.is_ok());
    }

    #[tokio::test]
    async fn parse_user_search_input_assorted() {
        game::search::parse_user_input("test", None);
        game::search::parse_user_input(r#"tag:"sonic""#, None);
        game::search::parse_user_input(r#"o_%$ dev:"san" disk t:7 potato"#, None);

        enable_debug();

        // "" should be treated as exact
        // Allow key characters in quoted text
        let s = game::search::parse_user_input(r#"title:"" series:"sonic:hedgehog" -developer:"""#, None).search;
        assert!(s.filter.exact_whitelist.title.is_some());
        assert_eq!(s.filter.exact_whitelist.title.unwrap()[0], "");
        assert!(s.filter.whitelist.series.is_some());
        assert_eq!(s.filter.whitelist.series.unwrap()[0], "sonic:hedgehog");
        assert!(s.filter.exact_blacklist.developer.is_some());
        assert_eq!(s.filter.exact_blacklist.developer.unwrap()[0], "");

        // Make sure the number filters are populated and the time text is processes
        let s2 = game::search::parse_user_input(r#"playtime>1h30m tags:3 playcount<3"#, None).search;
        assert!(s2.filter.higher_than.playtime.is_some());
        assert_eq!(s2.filter.higher_than.playtime.unwrap(), 60 * 90);
        assert!(s2.filter.equal_to.tags.is_some());
        assert_eq!(s2.filter.equal_to.tags.unwrap(), 3);
        assert!(s2.filter.lower_than.playcount.is_some());
        assert_eq!(s2.filter.lower_than.playcount.unwrap(), 3);
    }

    #[tokio::test]
    async fn parse_user_search_input_sizes() {
        let search = game::search::parse_user_input("tags>5 addapps=3 gamedata<12 test>generic", None).search;
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
    async fn game_redirects() {
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
        let game = result.unwrap();

        let create_redirect_res = flashpoint.create_game_redirect("test", &game.id).await;
        assert!(create_redirect_res.is_ok());

        // Find game redirect
        let found_game_res = flashpoint.find_game("test").await;
        assert!(found_game_res.is_ok());
        assert!(found_game_res.unwrap().is_some());

        // ID search redirect
        let mut search = GameSearch::default();
        search.filter.exact_whitelist.id = Some(vec!["test".to_owned()]);
        let search_res = flashpoint.search_games(&search).await;
        assert!(search_res.is_ok());
        assert_eq!(search_res.unwrap().len(), 1);

        // Find redirects
        let found_redirs = flashpoint.find_game_redirects().await;
        assert!(found_redirs.is_ok());
        assert_eq!(found_redirs.unwrap().len(), 1);

        let remove_redirect_res = flashpoint.delete_game_redirect("test", &game.id).await;
        assert!(remove_redirect_res.is_ok());

        let found_redirs2 = flashpoint.find_game_redirects().await;
        assert!(found_redirs2.is_ok());
        assert_eq!(found_redirs2.unwrap().len(), 0);
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
    async fn game_extension() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let create_ext = flashpoint.register_extension(ExtensionInfo { 
            id: "user_score".to_owned(),
            searchables: vec![ExtSearchable {
                key: "score".to_owned(),
                search_key: "score".to_owned(),
                value_type: game::ext::ExtSearchableType::Number
            }],
            indexes: vec![] 
        });
        assert!(create_ext.is_ok());

        // Save some game info with ext data
        let partial_game = game::PartialGame {
            title: Some(String::from("Test Game")),
            tags: Some(vec!["Action"].into()),
            ..game::PartialGame::default()
        };
        let game_create_res = flashpoint.create_game(&partial_game).await;
        assert!(game_create_res.is_ok());
        let mut game = game_create_res.unwrap();
        let mut ext_map = HashMap::new();
        let ext_data = serde_json::from_str(r#"{"score": 5}"#);
        assert!(ext_data.is_ok());
        ext_map.insert("user_score".to_owned(), ext_data.unwrap());
        game.ext_data = Some(ext_map);
        let save_res = flashpoint.save_game(&mut game.into()).await;
        assert!(save_res.is_ok());

        // Search for this game
        let search = parse_user_input("score>3", Some(&flashpoint.extensions.searchables)).search;
        let search_res = flashpoint.search_games(&search).await;
        assert!(search_res.is_ok());
        let res = search_res.unwrap();
        assert_eq!(res.len(), 1);

        let search = parse_user_input("score<3", Some(&flashpoint.extensions.searchables)).search;
        let search_res = flashpoint.search_games(&search).await;
        assert!(search_res.is_ok());
        let res = search_res.unwrap();
        assert_eq!(res.len(), 0);
    }

    #[tokio::test]
    async fn game_extension_user_input() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let create_ext = flashpoint.register_extension(ExtensionInfo { 
            id: "user_score".to_owned(),
            searchables: vec![
            ExtSearchable {
                key: "renamed".to_owned(),
                search_key: "name".to_owned(),
                value_type: game::ext::ExtSearchableType::String,
            },
            ExtSearchable {
                key: "fav".to_owned(),
                search_key: "fav".to_owned(),
                value_type: game::ext::ExtSearchableType::Boolean,
            },
            ExtSearchable {
                key: "score".to_owned(),
                search_key: "score".to_owned(),
                value_type: game::ext::ExtSearchableType::Number
            }],
            indexes: vec![],
        });
        assert!(create_ext.is_ok());
        let search = parse_user_input("score>5 name:sonic fav=1", Some(&flashpoint.extensions.searchables)).search;

        // Number field
        assert!(search.filter.higher_than.ext.is_some());
        let ext_search = search.filter.higher_than.ext.unwrap();
        assert!(ext_search.contains_key("user_score"));
        let ext_search_entry = ext_search.get("user_score").unwrap();
        assert!(ext_search_entry.contains_key("score"));
        let ext_search_entry_score = ext_search_entry.get("score").unwrap();
        assert_eq!(*ext_search_entry_score, 5);

        // Bool field
        assert!(search.filter.bool_comp.ext.is_some());
        let ext_search = search.filter.bool_comp.ext.unwrap();
        assert!(ext_search.contains_key("user_score"));
        let ext_search_entry = ext_search.get("user_score").unwrap();
        assert!(ext_search_entry.contains_key("fav"));
        let ext_search_entry_score = ext_search_entry.get("fav").unwrap();
        assert_eq!(*ext_search_entry_score, true);

        // String field
        assert!(search.filter.whitelist.ext.is_some());
        let ext_search = search.filter.whitelist.ext.unwrap();
        assert!(ext_search.contains_key("user_score"));
        let ext_search_entry = ext_search.get("user_score").unwrap();
        assert!(ext_search_entry.contains_key("renamed"));
        let ext_search_entry_score = ext_search_entry.get("renamed").unwrap();
        assert!(ext_search_entry_score.iter().find(|&s| *s == "sonic").is_some());
    }

    #[tokio::test]
    async fn create_and_save_game_data() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let partial_game = game::PartialGame {
            title: Some(String::from("Test Game")),
            tags: Some(vec!["Action"].into()),
            ..game::PartialGame::default()
        };
        let game_create_res = flashpoint.create_game(&partial_game).await;
        assert!(game_create_res.is_ok());
        let game = game_create_res.unwrap();
        let game_data = PartialGameData { 
            id: None,
            game_id: game.id,
            title: Some("Test".to_owned()),
            date_added: Some("2023-01-01T01:01:01.000".to_owned()),
            sha256: Some("123".to_owned()),
            crc32: Some(0),
            present_on_disk: Some(false),
            path: None,
            size: Some(123),
            parameters: None,
            application_path: Some("Test".to_owned()),
            launch_command: Some("Test".to_owned())
        };

        let game_data_res = flashpoint.create_game_data(&game_data).await;
        assert!(game_data_res.is_ok());
        let mut gd = game_data_res.unwrap();
        gd.path = Some("Test".to_owned());
        let save_res = flashpoint.save_game_data(&gd.into()).await;
        assert!(save_res.is_ok());
        let new_gd = save_res.unwrap();
        assert_eq!(new_gd.path.unwrap(), "Test");
    }

    #[tokio::test]
    async fn parse_user_search_input() {
        let input = r#"sonic title:"dog cat" -title:"cat dog" tag:Action -mario installed:true"#;
        let search = game::search::parse_user_input(input, None).search;
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
        assert!(search.filter.bool_comp.installed.is_some());
        assert_eq!(search.filter.bool_comp.installed.unwrap(), true);
    }

    #[tokio::test]
    async fn parse_user_search_input_whitespace() {
        let input = r#"series:"紅白Flash合戦  / Red & White Flash Battle 2013""#;
        let search = game::search::parse_user_input(input, None).search;
        assert!(search.filter.whitelist.series.is_some());
        assert_eq!(search.filter.whitelist.series.unwrap()[0], "紅白Flash合戦  / Red & White Flash Battle 2013");
    }

    #[tokio::test]
    async fn parse_user_quick_search_input() {
        let input = r#"#Action -!Flash @"armor games" !"#;
        let search = game::search::parse_user_input(input, None).search;
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
        let input = r#"!Flash -publisher=Newgrounds =sonic"#;
        let search = game::search::parse_user_input(input, None).search;
        assert!(search.filter.whitelist.platforms.is_some());
        assert_eq!(search.filter.whitelist.platforms.unwrap()[0], "Flash");
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
    async fn find_tag() {
        let mut flashpoint = FlashpointArchive::new();
        assert!(flashpoint.load_database(":memory:").is_ok());
        let partial = PartialGame {
            title: Some("test".to_owned()),
            tags: Some(vec!["Action"].into()),
            ..Default::default()
        };
        let new_game_res = flashpoint.create_game(&partial).await;
        assert!(new_game_res.is_ok());
        let tag_res = flashpoint.find_tag("Action").await;
        assert!(tag_res.is_ok());
        let tag_opt = tag_res.unwrap();
        assert!(tag_opt.is_some());
        let tag_id_res = flashpoint.find_tag_by_id(tag_opt.unwrap().id).await;
        assert!(tag_id_res.is_ok());
        assert!(tag_id_res.unwrap().is_some());
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
        platform.name = String::from("Wiggle");
        let mut partial = PartialTag::from(platform);
        let save_res = flashpoint.save_platform(&mut partial).await;
        assert!(save_res.is_ok());
        assert_eq!(save_res.unwrap().name, "Wiggle");
        let new_game = flashpoint.find_game(&old_game.id).await.unwrap().unwrap();
        assert_eq!(new_game.primary_platform, "Wiggle");
        assert!(new_game.platforms.contains(&"Wiggle".to_string()));
    }

    #[tokio::test]
    async fn search_games_random() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());

        let mut search = crate::game::search::parse_user_input("", None).search;
        let mut new_filter = GameFilter::default();
        new_filter.exact_blacklist.tags = Some(vec!["Action".to_owned()]);
        search.filter.subfilters.push(new_filter);

        let random_res = flashpoint.search_games_random(&search, 5).await;
        assert!(random_res.is_ok());
        assert_eq!(random_res.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn search_games_installed() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());

        let mut search = crate::game::search::parse_user_input("installed:true", None).search;
        if let Some(installed) = search.filter.bool_comp.installed.as_ref() {
            assert_eq!(installed, &true);
        } else {
            panic!("Expected 'installed' to be Some(true), but it was None.");
        }

        search.limit = 200;
        let games_res = flashpoint.search_games(&search).await;
        assert!(games_res.is_ok());
        assert_eq!(games_res.unwrap().len(), 20);
    }

    #[tokio::test]
    async fn search_games_index_limited() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());

        let search = &mut GameSearch::default();
        search.filter.whitelist.title = Some(vec!["Super".into()]);
        // Set page size
        search.limit = 200;
        let index_res = flashpoint.search_games_index(&mut search.clone(), Some(1000)).await;
        assert!(index_res.is_ok());
        let index = index_res.unwrap();
        assert_eq!(index.len(), 5);
    }

    
    #[tokio::test]
    async fn search_bracketting() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());

        let search = &mut GameSearch::default();

        let mut tag_filter = GameFilter::default();
        tag_filter.whitelist.tags = Some(vec!["Alien Hominid".into()]);

        let mut dev_filter = GameFilter::default();
        dev_filter.whitelist.developer = Some(vec!["jmtb".into(), "Tom Fulp".into()]);
        dev_filter.match_any = true;

        search.filter.match_any = false;
        search.filter.subfilters.push(dev_filter);
        search.filter.subfilters.push(tag_filter);

        let games_res = flashpoint.search_games(&search).await;
        assert!(games_res.is_ok());
        assert_eq!(games_res.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_tag() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());

        let tag_res = flashpoint.find_tag("Mario Bros.").await;
        assert!(tag_res.is_ok());
        let tag = tag_res.unwrap();
        assert!(tag.is_some());
        assert_eq!(tag.unwrap().name, "Super Mario");
    }

    #[tokio::test]
    async fn get_platform() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());

        let tag_res = flashpoint.find_platform("Jutvision").await;
        assert!(tag_res.is_ok());
        let tag = tag_res.unwrap();
        assert!(tag.is_some());
        assert_eq!(tag.unwrap().name, "asdadawdaw");
    }

    #[tokio::test]
    async fn add_playtime() {
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
        let game_id = result.unwrap().id;
        let playtime_res = flashpoint.add_game_playtime(&game_id, 30).await;
        assert!(playtime_res.is_ok());
        let saved_game_res = flashpoint.find_game(&game_id).await;
        assert!(saved_game_res.is_ok());
        let saved_game_opt = saved_game_res.unwrap();
        assert!(saved_game_opt.is_some());
        let saved_game = saved_game_opt.unwrap();
        assert_eq!(saved_game.playtime, 30);
        assert_eq!(saved_game.play_counter, 1);
    }

    #[tokio::test]
    async fn update_tags_clear_existing() {
        let mut flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let new_tag_res = flashpoint.create_tag("test", None, Some(10)).await;
        assert!(new_tag_res.is_ok());
        let tag_update = RemoteTag {
            id: 10,
            name: "hello".to_owned(),
            description: String::new(),
            category: "default".to_owned(),
            date_modified: "2024-01-01 12:00:00".to_owned(),
            aliases: vec!["hello".to_owned()],
            deleted: false,
        };
        let update_res = flashpoint.update_apply_tags(vec![tag_update]).await;
        assert!(update_res.is_ok());
        let saved_tag_res = flashpoint.find_tag_by_id(10).await;
        assert!(saved_tag_res.is_ok());
        let saved_tag_opt = saved_tag_res.unwrap();
        assert!(saved_tag_opt.is_some());
        let saved_tag = saved_tag_opt.unwrap();
        assert_eq!(saved_tag.aliases.len(), 1);
        assert_eq!(saved_tag.aliases[0].as_str(), "hello");
        assert_eq!(saved_tag.name.as_str(), "hello");
    }
}
