use game::PartialGame;
use rusqlite::Connection;
use snafu::ResultExt;
use std::sync::Mutex;

mod error;
use error::{Error, Result};

mod game;
mod game_data;
mod migration;
mod platform;
mod tag;

pub struct Flashpoint {
    conn: Mutex<Option<Connection>>,
}

impl Flashpoint {
    pub fn new() -> Flashpoint {
        Flashpoint {
            conn: Mutex::new(None),
        }
    }

    pub fn load_database(&self, source: &str) -> Result<()> {
        let mut conn_lock = self.conn.lock().unwrap();
        if let Some(conn) = conn_lock.take() {
            conn.close()
                .map_err(|e| Error::SqliteError { source: e.1 })?;
        }
        if source == ":memory:" {
            let mut conn = Connection::open_in_memory().context(error::SqliteSnafu)?;
            migration::up(&mut conn).context(error::DatabaseMigrationSnafu)?;
            conn.execute("PRAGMA foreign_keys=off;", ()).context(error::SqliteSnafu)?;
            rusqlite::vtab::array::load_module(&conn).context(error::SqliteSnafu)?;
            *conn_lock = Some(conn);
            Ok(())
        } else {
            let mut conn = Connection::open(source).context(error::SqliteSnafu)?;
            migration::up(&mut conn).context(error::DatabaseMigrationSnafu)?;
            conn.execute("PRAGMA foreign_keys=off;", ()).context(error::SqliteSnafu)?;
            rusqlite::vtab::array::load_module(&conn).context(error::SqliteSnafu)?;
            *conn_lock = Some(conn);
            Ok(())
        }
    }

    pub fn find_game(&self, id: &str) -> Result<Option<game::Game>> {
        with_connection!(self.conn, |conn| {
            game::find(conn, id).context(error::SqliteSnafu)
        })
    }

    pub fn create_game(&self, partial_game: &PartialGame) -> Result<game::Game> {
        with_connection!(self.conn, |conn| {
            game::create(conn, partial_game).context(error::SqliteSnafu)
        })
    }

    pub fn save_game(&self, partial_game: &PartialGame) -> Result<game::Game> {
        with_connection!(self.conn, |conn| {
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

    pub fn find_tag(&self, name: &str) -> Result<Option<tag::Tag>> {
        with_connection!(self.conn, |conn| {
            tag::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub fn count_tags(&self) -> Result<i64> {
        with_connection!(self.conn, |conn| {
            tag::count(conn).context(error::SqliteSnafu)
        })
    }

    pub fn find_platform(&self, name: &str) -> Result<Option<tag::Tag>> {
        with_connection!(self.conn, |conn| {
            platform::find_by_name(conn, name).context(error::SqliteSnafu)
        })
    }

    pub fn count_platforms(&self) -> Result<i64> {
        with_connection!(self.conn, |conn| {
            platform::count(conn).context(error::SqliteSnafu)
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

#[cfg(test)]
mod tests {
    use crate::game::PartialGame;

    use super::*;

    #[test]
    fn database_not_initialized() {
        let flashpoint = Flashpoint::new();
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
        let flashpoint = Flashpoint::new();
        let create = flashpoint.load_database("flashpoint.sqlite");
        assert!(create.is_ok());
        let result = flashpoint.count_games();
        assert!(result.is_ok());

        let total = result.unwrap();
        assert_eq!(total, 191150);
    }

    #[test]
    fn find_game() {
        let flashpoint = Flashpoint::new();
        let create = flashpoint.load_database("flashpoint.sqlite");
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
    fn create_and_save_game() {
        let flashpoint = Flashpoint::new();
        let create = flashpoint.load_database(":memory:");
        assert!(create.is_ok());
        let partial_game = PartialGame {
            title: Some(String::from("Test Game")),
            ..PartialGame::default()
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
}
