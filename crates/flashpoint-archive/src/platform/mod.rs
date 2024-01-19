use rusqlite::{Connection, Result, params};

use crate::{tag::Tag, game::{self, search::GameSearchRelations}};

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

pub fn create(conn: &mut Connection, name: &str) -> Result<Tag> {
    // Create the alias
    let mut stmt = "INSERT INTO platform_alias (name, platformId) VALUES(?, ?) RETURNING id";    
    let tx = conn.transaction()?;

    // Create a new tag
    let alias_id: i64 = tx.query_row(stmt, params![name, -1], |row| row.get(0))?;

    stmt = "INSERT INTO platform (primaryAliasId, description) VALUES (?, ?) RETURNING id";
    let tag_id: i64 = tx.query_row(stmt, params![alias_id, ""], |row| row.get(0))?;

    // Update tag alias with the new tag id
    stmt = "UPDATE platform_alias SET platformId = ? WHERE id = ?";
    tx.execute(stmt, params![tag_id, alias_id])?;

    tx.commit()?;

    let new_tag_result = find_by_name(conn, name)?;
    if let Some(tag) = new_tag_result {
        Ok(tag)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn find_or_create(conn: &mut Connection, name: &str) -> Result<Tag> {
    let platform_result = find_by_name(conn, name)?;
    if let Some(platform) = platform_result {
        Ok(platform)
    } else {
        create(conn, name)
    }
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform_alias pa
        INNER JOIN platform p ON p.id = pa.platformId
        WHERE pa.name = ? AND p.primaryAliasId == pa.id")?;

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

pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, pa.name, p.description, p.dateModified FROM platform_alias pa
        INNER JOIN platform p ON p.id = pa.platformId
        WHERE pa.name = ? AND p.primaryAliasId == pa.id")?;

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

pub fn delete(conn: &mut Connection, name: &str) -> Result<()> {
    let tag = find_by_name(conn, name)?;
    match tag {
        Some(tag) => {
            println!("Found tag");
            let mut search = game::search::parse_user_input("");
            search.limit = 99999999;
            search.load_relations = GameSearchRelations {
                tags: false,
                platforms: true,
                add_apps: false,
                game_data: false
            };
            let games = game::search::search(conn, &search)?;
            println!("{} games", games.len());

            let tx = conn.transaction()?;

            // Remove platform from games
            for game in games {
                let new_tags = game.detailed_platforms.unwrap().iter().filter(|t| t.name != name).map(|t| t.name.clone()).collect::<Vec<String>>();
                tx.execute("UPDATE game SET platformsStr = ? WHERE id = ?", params![new_tags.join("; "), game.id])?;
            }

            let mut stmt = "DELETE FROM game_platforms_platform WHERE platformId = ?";
            tx.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM platform_alias WHERE platformId = ?";
            tx.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM platform WHERE id = ?";
            tx.execute(stmt, params![tag.id])?;

            tx.commit()?;

            Ok(())
        },
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}
