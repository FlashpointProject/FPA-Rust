use rusqlite::{Connection, Result, params};

use crate::tag::Tag;

pub fn count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM platform", (), |row| {
        row.get::<_, i64>(0)
    })
}

pub fn find(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform_alias pa
        INNER JOIN platform p ON p.id = pa.platformId")?;

    let platform_iter = stmt.query_map((), |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            date_modified: row.get(3)?,
            aliases: vec![],
            category: None,
        })
    })?;

    let mut platforms = vec![];

    for platform in platform_iter {
        let mut platform = platform?;
        let mut platform_alias_stmt = conn.prepare(
            "SELECT pa.name FROM platform_alias ta WHERE pa.platformId = ?")?;
        let platform_alias_iter = platform_alias_stmt.query_map(params![&platform.id], |row| row.get(0))?;
        
        for alias in platform_alias_iter {
            platform.aliases.push(alias.unwrap());
        }
        platforms.push(platform);
    }

    Ok(platforms)
}

pub fn find_or_create(conn: &Connection, name: &str) -> Result<Tag> {
    let platform_result = find_by_name(conn, name)?;
    if let Some(platform) = platform_result {
        Ok(platform)
    } else {
        // Create the alias
        let mut stmt = conn.prepare(
            "INSERT INTO platform_alias (name, platformId) VALUES(?, ?) RETURNING id"
        )?;

        // Create a new tag
        let alias_id: i64 = stmt.query_row(params![name, -1], |row| row.get(0))?;
        stmt = conn.prepare(
            "INSERT INTO platform (primaryAliasId, description) VALUES (?, '') RETURNING id"
        )?;
        let tag_id: i64 = stmt.query_row(params![alias_id], |row| row.get(0))?;

        // Update tag alias with the new tag id
        stmt = conn.prepare(
            "UPDATE platform_alias SET platformId = ? WHERE id = ?"
        )?;
        stmt.execute(params![tag_id, alias_id])?;

        let new_platform_result = find_by_name(conn, name)?;
        if let Some(platform) = new_platform_result {
            Ok(platform)
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }

    }
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform_alias pa
        INNER JOIN platform p ON p.id = pa.platformId
        WHERE pa.name = ?")?;

    let platform_result = stmt.query_row(params![name], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            date_modified: row.get(3)?,
            category: None,
            aliases: vec![],
        })
    });

    match platform_result {
        Ok(mut platform) => {
            let mut platform_alias_stmt = conn.prepare(
                "SELECT pa.name FROM platform_alias pa WHERE pa.platformId = ?")?;
            let platform_alias_iter = platform_alias_stmt.query_map(params![&platform.id], |row| row.get(0))?;
            
            for alias in platform_alias_iter {
                platform.aliases.push(alias.unwrap());
            }

            Ok(Some(platform))
        },
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}