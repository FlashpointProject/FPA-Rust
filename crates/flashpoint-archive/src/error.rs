use chrono::ParseError;
use rusqlite;
use snafu::prelude::*;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Database not initialized"))]
    DatabaseNotInitialized,
    #[snafu(display("Database failed to migrate: {}", source))]
    DatabaseMigration { source: rusqlite_migration::Error },
    #[snafu(display("Invalid table name: {}", table_name))]
    InvalidTableName { table_name: String },
    #[snafu(display("SQLite error: {}", source))]
    SqliteError { source: rusqlite::Error },
    #[snafu(display("Mutex lock failed"))]
    MutexLockFailed,
    #[snafu(display("Transaction already open"))]
    TransactionAlreadyOpen,
    #[snafu(display("Failed to parse date '{}': {}", date, source))]
    DateParseError {
        date: String,
        source: ParseError,
    },
    #[snafu(display("Error generating content tree"))]
    ContentTreeError,
    #[snafu(display("Error copying folder"))]
    CopyFolderError,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
