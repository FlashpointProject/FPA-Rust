use game::{PartialGame, search::GameSearch};
use rusqlite::Connection;
use snafu::ResultExt;
use tag::Tag;
use tag_category::{TagCategory, PartialTagCategory};
use std::sync::Mutex;

mod error;
use error::{Error, Result};

pub mod game;
mod game_data;
mod migration;
mod platform;
mod tag;
mod tag_category;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

pub struct FlashpointArchive {
    conn: Mutex<Option<Connection>>,
}

impl FlashpointArchive {
    pub fn new() -> FlashpointArchive {
        FlashpointArchive {
            conn: Mutex::new(None),
        }
    }

    /// Load a new database for Flashpoint. Open databases will close.
    /// 
    /// `source` - Path to database file, or :memory: to open a fresh database in memory
    pub fn load_database(&self, source: &str) -> Result<()> {
        let mut conn_lock = self.conn.lock().unwrap();
        if let Some(conn) = conn_lock.take() {
            conn.close()
                .map_err(|e| Error::SqliteError { source: e.1 })?;
        }

        let mut conn = if source == ":memory:" {
            Connection::open_in_memory().context(error::SqliteSnafu)?
        } else {
            Connection::open(source).context(error::SqliteSnafu)?
        };

        // Perform database migrations
        migration::up(&mut conn).context(error::DatabaseMigrationSnafu)?;
        conn.execute("PRAGMA foreign_keys=off;", ()).context(error::SqliteSnafu)?;
        // Always make there's always a default tag category present 
        tag_category::find_or_create(&conn, "default").context(error::SqliteSnafu)?;
        // Allow use of rarray() in SQL queries
        rusqlite::vtab::array::load_module(&conn).context(error::SqliteSnafu)?;
        *conn_lock = Some(conn);
        Ok(())
    }

    pub fn search_games(&self, search: &GameSearch) -> Result<Vec<game::Game>> {
        with_connection!(self.conn, |conn| {
            game::search::search(conn, search).context(error::SqliteSnafu)
        })
    }

    pub fn search_games_index(&self, search: &GameSearch) -> Result<Vec<String>> {
        with_connection!(self.conn, |conn| {
            game::search::search_index(conn, search).context(error::SqliteSnafu)
        })
    }

    pub fn search_games_total(&self, search: &GameSearch) -> Result<i64> {
        with_connection!(self.conn, |conn| {
            game::search::search_count(conn, search).context(error::SqliteSnafu)
        })
    }

    pub fn find_game(&self, id: &str) -> Result<Option<game::Game>> {
        with_connection!(self.conn, |conn| {
            game::find(conn, id).context(error::SqliteSnafu)
        })
    }

    pub fn create_game(&self, partial_game: &PartialGame) -> Result<game::Game> {
        with_mut_connection!(self.conn, |conn| {
            game::create(conn, partial_game).context(error::SqliteSnafu)
        })
    }

    pub fn save_game(&self, partial_game: &PartialGame) -> Result<game::Game> {
        with_mut_connection!(self.conn, |conn| {
            game::save(conn, partial_game).context(error::SqliteSnafu)
        })
    }

    pub fn delete_game(&self, id: &str) -> Result<usize> {
        with_connection!(self.conn, |conn| {
            game::delete(conn, id).context(error::SqliteSnafu)
        })
    }

    pub fn count_games(&self) -> Result<i64> {
        with_connection!(self.conn, |conn| {
            game::count(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_all_tags(&self) -> Result<Vec<Tag>> {
        with_connection!(self.conn, |conn| {
            tag::find(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_tag(&self, name: &str) -> Result<Option<Tag>> {
        with_connection!(self.conn, |conn| {
            tag::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub fn count_tags(&self) -> Result<i64> {
        with_connection!(self.conn, |conn| {
            tag::count(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_all_platforms(&self) -> Result<Vec<Tag>> {
        with_connection!(self.conn, |conn| {
            platform::find(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_platform(&self, name: &str) -> Result<Option<Tag>> {
        with_connection!(self.conn, |conn| {
            platform::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub fn count_platforms(&self) -> Result<i64> {
        with_connection!(self.conn, |conn| {
            platform::count(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_all_tag_categories(&self) -> Result<Vec<TagCategory>> {
        with_connection!(self.conn, |conn| {
            tag_category::find(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_tag_category(&self, name: &str) -> Result<Option<TagCategory>> {
        with_connection!(self.conn, |conn| {
            tag_category::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub fn create_tag_category(&self, partial: &PartialTagCategory) -> Result<TagCategory> {
        with_connection!(self.conn, |conn| {
            tag_category::create(conn, partial).context(error::SqliteSnafu)
        })
    }
}

#[macro_export]
macro_rules! with_connection {
    ($lock:expr, $body:expr) => {
        match $lock.lock() {
            Ok(guard) => {
                if let Some(ref conn) = *guard {
                    $body(conn)
                } else {
                    Err(Error::DatabaseNotInitialized {})
                }
            }
            Err(_) => Err(Error::MutexLockFailed {}),
        }
    };
}

#[macro_export]
macro_rules! with_mut_connection {
    ($lock:expr, $body:expr) => {
        match $lock.lock() {
            Ok(mut guard) => {
                if let Some(ref mut conn) = *guard {
                    $body(conn)
                } else {
                    Err(Error::DatabaseNotInitialized {})
                }
            }
            Err(_) => Err(Error::MutexLockFailed {}),
        }
    };
}

#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => (if ::std::cfg!(debug_assertions) { ::std::println!($($arg)*); })
}

#[cfg(test)]
mod tests {

    use crate::game::search::GameSearchOffset;

    use super::*;

    const TEST_DATABASE: &str = "benches/flashpoint.sqlite";

    #[test]
    fn database_not_initialized() {
        let flashpoint = FlashpointArchive::new();
        let result = flashpoint.count_games();
        assert!(result.is_err());

        let e = result.unwrap_err();
        assert!(matches!(e, Error::DatabaseNotInitialized {}));
    }

    #[test]
    fn migrations_valid() {
        let migrations = migration::get();
        assert!(migrations.validate().is_ok());
    }

    #[test]
    fn count_games() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let result = flashpoint.count_games();
        assert!(result.is_ok());

        let total = result.unwrap();
        assert_eq!(total, 191150);
    }

    #[test]
    fn search_full_scan() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        search.limit = 99999999999;
        search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
        let result = flashpoint.search_games(&search);
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 162929);
    }

    #[test]
    fn search_tags_or() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        search.limit = 99999999999;
        search.filter.match_any = true;
        search.filter.exact_whitelist.tags = Some(vec!["Action".to_owned(), "Adventure".to_owned()]);
        let result = flashpoint.search_games(&search);
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 36724);
    }

    #[test]
    fn search_tags_and() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let mut search = game::search::GameSearch::default();
        search.limit = 99999999999;
        search.filter.match_any = false;
        search.filter.exact_whitelist.tags = Some(vec!["Action".to_owned(), "Adventure".to_owned()]);
        let result = flashpoint.search_games(&search);
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 397);
    }

    #[test]
    fn search_tags_and_or_combined() {
        // Has 'Action' or 'Adventure', but is missing 'Sonic The Hedgehog'
        let flashpoint = FlashpointArchive::new();
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
        let total_result = flashpoint.search_games_total(&search);
        assert!(total_result.is_ok());
        let total = total_result.unwrap();
        assert_eq!(total, 36541);

        // Test first page results
        let result = flashpoint.search_games(&search);
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 30000);
        let page_end_game = games.last().unwrap();

        // Test index
        let index_result = flashpoint.search_games_index(&search);
        assert!(index_result.is_ok());
        let index = index_result.unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index[0], page_end_game.id);

        // Test last page results
        search.offset = Some(GameSearchOffset{
            value: page_end_game.title.clone(),
            game_id: page_end_game.id.clone(),
        });
        let last_result = flashpoint.search_games(&search);
        assert!(last_result.is_ok());
        let last_page = last_result.unwrap();
        assert_eq!(last_page.len(), 6541);
    }

    #[test]
    fn find_game() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(TEST_DATABASE);
        assert!(create.is_ok());
        let result = flashpoint.find_game("00deff25-5cd2-40d1-a0e7-151d82ce16c5");
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

    #[test]
    fn tag_categories() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let partial_tc = tag_category::PartialTagCategory {
            name: "test".to_owned(),
            color: "#FF00FF".to_owned(),
            description: Some("test".to_owned()),
        };
        assert!(flashpoint.create_tag_category(&partial_tc).is_ok());
        let saved_cat_result = flashpoint.find_tag_category("test");
        assert!(saved_cat_result.is_ok());
        let saved_cat_opt = saved_cat_result.unwrap();
        assert!(saved_cat_opt.is_some());
        let saved_cat = saved_cat_opt.unwrap();
        assert_eq!(saved_cat.name, "test");
        assert_eq!(saved_cat.color, "#FF00FF");
        assert!(saved_cat.description.is_some());
        assert_eq!(saved_cat.description.unwrap(), "test");

        let all_cats_result = flashpoint.find_all_tag_categories();
        assert!(all_cats_result.is_ok());
        let all_cats = all_cats_result.unwrap();
        // Default category always exists
        assert_eq!(all_cats.len(), 2);
    }

    #[test]
    fn create_and_save_game() {
        let flashpoint = FlashpointArchive::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let partial_game = game::PartialGame {
            title: Some(String::from("Test Game")),
            tags: Some(vec!["Action"].into()),
            ..game::PartialGame::default()
        };
        let result = flashpoint.create_game(&partial_game);
        assert!(result.is_ok());
        let mut game = result.unwrap();
        assert_eq!(game.title, "Test Game");
        game.developer = String::from("Newgrounds");
        game.tags = vec!["Action", "Adventure"].into();
        game.primary_platform = String::from("Flash");
        let save_result = flashpoint.save_game(&game.into());
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

    #[test]
    fn parse_user_search_input() {
        let input = r#"sonic title:"dog cat" -title:"cat dog" tag:Action -mario"#;
        let search = game::search::parse_user_input(input);
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

    #[test]
    fn parse_user_quick_search_input() {
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

    #[test]
    fn parse_user_exact_search_input() {
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
}
