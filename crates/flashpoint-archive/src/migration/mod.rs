use rusqlite::Connection;
use rusqlite_migration::{M, Migrations, Result};

pub fn get() -> Migrations<'static> {
    let migrations = Migrations::new(vec![
        M::up(r#"
            CREATE TABLE IF NOT EXISTS "tag_category" (
                "id"	integer NOT NULL,
                "name"	varchar NOT NULL COLLATE NOCASE,
                "color"	varchar NOT NULL,
                "description"	varchar,
                PRIMARY KEY("id" AUTOINCREMENT)
            );
            CREATE TABLE IF NOT EXISTS "tag_alias" (
                "id"	integer NOT NULL,
                "tagId"	integer,
                "name"	varchar NOT NULL COLLATE NOCASE,
                CONSTRAINT "UQ_34d6ff6807129b3b193aea26789" UNIQUE("name"),
                PRIMARY KEY("id" AUTOINCREMENT)
            );
            CREATE TABLE IF NOT EXISTS "tag" (
                "id"	integer NOT NULL,
                "dateModified"	datetime NOT NULL DEFAULT (datetime('now')),
                "primaryAliasId"	integer,
                "categoryId"	integer,
                "description"	varchar,
                CONSTRAINT "REL_3c002904ab97fb1b4e61e8493c" UNIQUE("primaryAliasId"),
                PRIMARY KEY("id" AUTOINCREMENT)
            );
            CREATE TABLE IF NOT EXISTS "additional_app" (
                "id"	varchar NOT NULL,
                "applicationPath"	varchar NOT NULL,
                "autoRunBefore"	boolean NOT NULL,
                "launchCommand"	varchar NOT NULL,
                "name"	varchar NOT NULL COLLATE NOCASE,
                "waitForExit"	boolean NOT NULL,
                "parentGameId"	varchar,
                PRIMARY KEY("id")
            );
            CREATE TABLE IF NOT EXISTS "game_tags_tag" (
                "gameId"	varchar NOT NULL,
                "tagId"	integer NOT NULL,
                PRIMARY KEY("gameId","tagId")
            );
            CREATE TABLE IF NOT EXISTS "game" (
                "id"	varchar NOT NULL,
                "parentGameId"	varchar,
                "title"	varchar NOT NULL,
                "alternateTitles"	varchar NOT NULL,
                "series"	varchar NOT NULL,
                "developer"	varchar NOT NULL,
                "publisher"	varchar NOT NULL,
                "dateAdded"	datetime NOT NULL,
                "dateModified"	datetime NOT NULL DEFAULT (datetime('now')),
                "broken"	boolean NOT NULL,
                "extreme"	boolean NOT NULL,
                "playMode"	varchar NOT NULL,
                "status"	varchar NOT NULL,
                "notes"	varchar NOT NULL,
                "source"	varchar NOT NULL,
                "applicationPath"	varchar NOT NULL,
                "launchCommand"	varchar NOT NULL,
                "releaseDate"	varchar NOT NULL,
                "version"	varchar NOT NULL,
                "originalDescription"	varchar NOT NULL,
                "language"	varchar NOT NULL,
                "library"	varchar NOT NULL,
                "orderTitle"	varchar NOT NULL,
                "activeDataId"	integer,
                "activeDataOnDisk"	boolean NOT NULL DEFAULT (0),
                "tagsStr"	varchar NOT NULL DEFAULT ('') COLLATE NOCASE,
                "platformsStr"	varchar,
                "platformId"	integer,
                "platformName"	varchar,
                "lastPlayed"	datetime,
                "playtime"	integer DEFAULT 0,
                "playCounter"	integer DEFAULT 0,
                "archiveState"	integer DEFAULT 2,
                "activeGameConfigId"	integer,
                "activeGameConfigOwner"	varchar COLLATE NOCASE,
                PRIMARY KEY("id"),
                CONSTRAINT "FK_45a9180069d42ac8231ff11acd0" FOREIGN KEY("parentGameId") REFERENCES "game"("id") ON DELETE NO ACTION ON UPDATE NO ACTION
            );
            CREATE TABLE IF NOT EXISTS "game_data" (
                "id"	integer NOT NULL,
                "gameId"	varchar,
                "title"	varchar NOT NULL,
                "dateAdded"	datetime NOT NULL,
                "sha256"	varchar NOT NULL,
                "crc32"	integer NOT NULL,
                "presentOnDisk"	boolean NOT NULL DEFAULT (0),
                "path"	varchar,
                "size"	integer NOT NULL,
                "parameters"	varchar,
                "applicationPath"	varchar,
                "launchCommand"	varchar,
                PRIMARY KEY("id" AUTOINCREMENT),
                CONSTRAINT "FK_8854ee113e5b5d9c43ff9ee1c8b" FOREIGN KEY("gameId") REFERENCES "game"("id") ON DELETE NO ACTION ON UPDATE NO ACTION
            );
            CREATE TABLE IF NOT EXISTS "platform_alias" (
                "id"	integer NOT NULL,
                "platformId"	integer,
                "name"	varchar NOT NULL COLLATE NOCASE,
                CONSTRAINT "UQ_platform_alias_name_unique" UNIQUE("name"),
                PRIMARY KEY("id" AUTOINCREMENT)
            );
            CREATE TABLE IF NOT EXISTS "platform" (
                "id"	integer NOT NULL,
                "dateModified"	datetime NOT NULL DEFAULT (datetime('now')),
                "primaryAliasId"	integer,
                "description"	varchar,
                CONSTRAINT "REL_platform_primary_alias_unique" UNIQUE("primaryAliasId"),
                PRIMARY KEY("id" AUTOINCREMENT)
            );
            CREATE TABLE IF NOT EXISTS "game_platforms_platform" (
                "gameId"	varchar NOT NULL,
                "platformId"	integer NOT NULL,
                PRIMARY KEY("gameId","platformId")
            );
            CREATE TABLE IF NOT EXISTS "game_config" (
                "id"	integer NOT NULL,
                "gameId"	varchar NOT NULL COLLATE NOCASE,
                "name"	varchar NOT NULL COLLATE NOCASE,
                "owner"	varchar NOT NULL COLLATE NOCASE,
                "middleware"	varchar,
                PRIMARY KEY("id" AUTOINCREMENT)
            );
            CREATE INDEX IF NOT EXISTS "IDX_34d6ff6807129b3b193aea2678" ON "tag_alias" (
                "name"
            );
            CREATE INDEX IF NOT EXISTS "IDX_6366e7093c3571f85f1b5ffd4f" ON "game_tags_tag" (
                "gameId"
            );
            CREATE INDEX IF NOT EXISTS "IDX_d12253f0cbce01f030a9ced11d" ON "game_tags_tag" (
                "tagId"
            );
            CREATE INDEX IF NOT EXISTS "IDX_gameTitle" ON "game" (
                "title"
            );
            CREATE INDEX IF NOT EXISTS "IDX_total" ON "game" (
                "library",
                "broken",
                "extreme"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_series" ON "game" (
                "library",
                "series"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_publisher" ON "game" (
                "library",
                "publisher"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_developer" ON "game" (
                "library",
                "developer"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_dateModified" ON "game" (
                "library",
                "dateModified"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_dateAdded" ON "game" (
                "library",
                "dateAdded"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_title" ON "game" (
                "library",
                "title"
            );
            CREATE INDEX IF NOT EXISTS "IDX_game_data_game_id" ON "game_data" (
                "gameId",
                "dateAdded"
            );
            CREATE INDEX IF NOT EXISTS "IDX_game_activeDataId" ON "game" (
                "activeDataId"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_lastPlayed" ON "game" (
                "library",
                "lastPlayed"
            );
            CREATE INDEX IF NOT EXISTS "IDX_lookup_playtime" ON "game" (
                "library",
                "playtime"
            );
            CREATE INDEX IF NOT EXISTS "IDX_game_config_game_id" ON "game_config" (
                "gameId"
            );
            "#),
        M::up(r#"
            UPDATE platform
            SET dateModified = REPLACE(SUBSTR(dateModified, 1, 19), 'T', ' ') || '.' || SUBSTR(dateModified, 21, 3)
            WHERE dateModified LIKE '____-__-__T__:__:__.__%';
            UPDATE tag
            SET dateModified = REPLACE(SUBSTR(dateModified, 1, 19), 'T', ' ') || '.' || SUBSTR(dateModified, 21, 3)
            WHERE dateModified LIKE '____-__-__T__:__:__.__%';
            "#),
    ]);

    migrations
}

pub fn up(conn: &mut Connection) -> Result<()> {
    let migrations = get();

    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

    migrations.to_latest(conn)
}