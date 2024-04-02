use std::rc::Rc;

use rusqlite::{params, types::Value, Connection, OptionalExtension, Result};

use crate::tag::{PartialTag, Tag, TagSuggestion};

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct PlatformAppPath {
    pub app_path: String,
    pub count: i64,
}

pub fn count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM platform", (), |row| {
        row.get::<_, i64>(0)
    })
}

pub fn find(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform_alias pa
        INNER JOIN platform p ON p.id = pa.platformId
        WHERE pa.id == p.primaryAliasId")?;

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
            "SELECT ta.name FROM platform_alias ta WHERE ta.platformId = ?")?;
        let platform_alias_iter = platform_alias_stmt.query_map(params![&platform.id], |row| row.get(0))?;
        
        for alias in platform_alias_iter {
            platform.aliases.push(alias.unwrap());
        }
        platforms.push(platform);
    }

    Ok(platforms)
}

pub fn create(conn: &Connection, name: &str, id: Option<i64>) -> Result<Tag> {
    // Create the alias
    let mut stmt = "INSERT INTO platform_alias (name, platformId) VALUES(?, ?) RETURNING id";    

    // Create a new tag
    let alias_id: i64 = conn.query_row(stmt, params![name, -1], |row| row.get(0))?;

    match id {
        Some(id) => {
            stmt = "INSERT INTO platform (id, primaryAliasId, description) VALUES (?, ?, ?)";
            conn.execute(stmt, params![id, alias_id, ""])?;
        
            // Update tag alias with the new tag id
            stmt = "UPDATE platform_alias SET platformId = ? WHERE id = ?";
            conn.execute(stmt, params![id, alias_id])?;
        }
        None => {
            stmt = "INSERT INTO platform (primaryAliasId, description) VALUES (?, ?) RETURNING id";
            let tag_id: i64 = conn.query_row(stmt, params![alias_id, ""], |row| row.get(0))?;
        
            // Update tag alias with the new tag id
            stmt = "UPDATE platform_alias SET platformId = ? WHERE id = ?";
            conn.execute(stmt, params![tag_id, alias_id])?;
        }
    }


    let new_tag_result = find_by_name(conn, name)?;
    if let Some(tag) = new_tag_result {
        Ok(tag)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn find_or_create(conn: &Connection, name: &str, id: Option<i64>) -> Result<Tag> {
    let platform_result = find_by_name(conn, name)?;
    if let Some(platform) = platform_result {
        Ok(platform)
    } else {
        // Clear a lingering alias
        conn.execute("DELETE FROM platform_alias WHERE name = ?", params![name])?;
        create(conn, name, id)
    }
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform p
        INNER JOIN platform_alias pa ON p.id = pa.platformId
        WHERE p.id IN (SELECT alias.platformId FROM platform_alias alias WHERE alias.name = ?)
		AND p.primaryAliasId = pa.id")?;

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

pub fn  find_by_id(conn: &Connection, id: i64) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform_alias pa
        INNER JOIN platform p ON p.id = pa.platformId
        WHERE p.id = ? AND p.primaryAliasId == pa.id")?;

    let platform_result = stmt.query_row(params![id], |row| {
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

pub fn save(conn: &Connection, partial: &PartialTag) -> Result<Tag> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    let mut tag = match find_by_id(conn, partial.id)? {
        Some(t) => t,
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    };

    let mut new_tag_aliases = vec![];

    if tag.name != partial.name {
        // Update game primary fields
        let stmt = "UPDATE game
        SET platformName = ?
        WHERE game.id IN (
            SELECT gameId FROM game_platforms_platform WHERE platformId = ?   
        )";
        conn.execute(stmt, params![partial.name, tag.id])?;
    }

    tag.apply_partial(partial);

    let mut stmt = conn.prepare("SELECT platformId FROM platform_alias WHERE name = ?")?;

    // Check for collisions before updating
    for alias in tag.aliases.clone() {
        let existing_platform_id = stmt.query_row(params![alias], |row| row.get::<_, i64>(0)).optional()?;
        match existing_platform_id {
            Some(id) => {
                if id != tag.id {
                    return Err(rusqlite::Error::QueryReturnedNoRows) // TODO: Make this a proper error
                }
            },
            None => {
                new_tag_aliases.push(alias);
            }
        }
    }

    // Apply flat edits
    stmt = conn.prepare("UPDATE platform SET description = ?, dateModified = ? WHERE id = ?")?;
    stmt.execute(params![tag.description, tag.date_modified, tag.id])?;

    // Remove old aliases
    let mut stmt = "DELETE FROM platform_alias WHERE platformId = ? AND name NOT IN rarray(?)";
    let alias_rc = Rc::new(tag.aliases.iter().map(|v| Value::from(v.clone())).collect::<Vec<Value>>());
    conn.execute(stmt, params![tag.id, alias_rc])?;

    // Add new aliases
    for alias in new_tag_aliases {
        stmt = "INSERT INTO platform_alias (name, platformId) VALUES (?, ?)";
        conn.execute(stmt, params![alias, tag.id])?;
    }

    // Update primary alias id
    stmt = "UPDATE platform SET primaryAliasId = (SELECT id FROM platform_alias WHERE name = ?) WHERE id = ?";
    conn.execute(stmt, params![tag.name, tag.id])?;

    // Update game platformsStr fields
    stmt = "UPDATE game
    SET platformsStr = (
        SELECT IFNULL(string_agg(pa.name, '; '), '')
        FROM game_platforms_platform gpp
        JOIN platform p ON gpp.platformId = p.id
        JOIN platform_alias pa ON p.primaryAliasId = pa.id
        WHERE gpp.gameId = game.id
    ) WHERE game.id IN (
        SELECT gameId FROM game_platforms_platform WHERE platformId = ?   
    )";
    conn.execute(stmt, params![tag.id])?;

    match find_by_id(&conn, tag.id)? {
        Some(t) => Ok(t),
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn delete(conn: &Connection, name: &str) -> Result<()> {
    let tag = find_by_name(conn, name)?;
    match tag {
        Some(tag) => {
            let mut stmt = "DELETE FROM platform_alias WHERE platformId = ?";
            conn.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM platform WHERE id = ?";
            conn.execute(stmt, params![tag.id])?;

            stmt = "UPDATE game
            SET platformName = ?
            WHERE game.id IN (
                SELECT gameId FROM game_platforms_platform WHERE platformId = ?   
            )";
            conn.execute(stmt, params!["", tag.id])?;

            // Update game platformsStr fields
            stmt = "UPDATE game
            SET platformsStr = (
                SELECT IFNULL(string_agg(pa.name, '; '), '')
                FROM game_platforms_platform gpp
                JOIN platform p ON gpp.platformId = p.id
                JOIN platform_alias pa ON p.primaryAliasId = pa.id
                WHERE gpp.gameId = game.id
            ) WHERE game.id IN (
                SELECT gameId FROM game_platforms_platform WHERE platformId = ?   
            )";
            conn.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM game_platforms_platform WHERE platformId = ?";
            conn.execute(stmt, params![tag.id])?;

            Ok(())
        },
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

pub fn search_platform_suggestions(
    conn: &Connection,
    partial: &str,
) -> Result<Vec<TagSuggestion>> {
    let mut suggestions = vec![];

    let query = "SELECT sugg.tagId, sugg.matched_alias, count(game_tag.gameId) as gameCount, sugg.primary_alias FROM (
        SELECT 
			ta1.platformId as tagId,
			ta1.name AS matched_alias,
			ta2.name AS primary_alias
		FROM 
			platform_alias ta1
		JOIN 
        platform t ON ta1.platformId = t.id
		JOIN 
        platform_alias ta2 ON t.primaryAliasId = ta2.id
		WHERE 
			ta1.name LIKE ?
    ) sugg
    LEFT JOIN game_platforms_platform game_tag ON game_tag.platformId = sugg.tagId
    GROUP BY sugg.matched_alias
    ORDER BY COUNT(game_tag.gameId) DESC, sugg.matched_alias ASC";

    let mut stmt = conn.prepare(&query)?;
    let mut likeable = String::from(partial);
    likeable.push_str("%");
    let results = stmt.query_map(params![&likeable], |row| {
        Ok(TagSuggestion {
            id: row.get(0)?,
            matched_from: row.get(1)?,
            games_count: row.get(2)?,
            name: row.get(3)?,
            category: None,
        })
    })?;

    for sugg in results {
        suggestions.push(sugg?);
    }

    Ok(suggestions)
}
