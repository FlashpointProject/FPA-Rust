use std::rc::Rc;

use chrono::NaiveDateTime;
use rusqlite::{Connection, Result, params, OptionalExtension, types::Value};

use crate::tag_category;

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub date_modified: NaiveDateTime,
    pub aliases: Vec<String>,
    pub category: Option<String>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct PartialTag {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub date_modified: Option<NaiveDateTime>,
    pub aliases: Option<Vec<String>>,
    pub category: Option<String>,
}

impl Tag {
    fn apply_partial(&mut self, partial: &PartialTag) {
        self.name = partial.name.clone();

        if let Some(aliases) = partial.aliases.clone() {
            self.aliases = aliases;
            if !self.aliases.contains(&self.name) {
                self.aliases.push(self.name.clone());
            }
        }

        if let Some(description) = partial.description.clone() {
            self.description = description;
        }

        if let Some(date_modified) = partial.date_modified {
            self.date_modified = date_modified;
        }

        if let Some(category) = partial.category.clone() {
            self.category = Some(category);
        }
    }
}

pub fn find(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag_alias ta
        INNER JOIN tag t ON t.id = ta.tagId
        INNER JOIN tag_category tc ON t.categoryId = tc.id")?;

    let tag_iter = stmt.query_map((), |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            date_modified: row.get(3)?,
            aliases: vec![],
            category: row.get(4)?,
        })
    })?;

    let mut tags = vec![];

    for tag in tag_iter {
        let mut tag = tag?;
        let mut tag_alias_stmt = conn.prepare(
            "SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;
        let tag_alias_iter = tag_alias_stmt.query_map(params![&tag.id], |row| row.get(0))?;
        
        for alias in tag_alias_iter {
            tag.aliases.push(alias.unwrap());
        }
        tags.push(tag);
    }

    Ok(tags)
}

pub fn create(conn: &mut Connection, name: &str, category: Option<String>) -> Result<Tag> {
    // Create the alias
    let mut stmt = "INSERT INTO tag_alias (name, tagId) VALUES(?, ?) RETURNING id";
    let category = tag_category::find_or_create(conn, category.unwrap_or_else(|| "default".to_owned()).as_str(), None)?;

    let tx = conn.transaction()?;

    // Create a new tag
    let alias_id: i64 = tx.query_row(stmt, params![name, -1], |row| row.get(0))?;

    stmt = "INSERT INTO tag (primaryAliasId, description, categoryId) VALUES (?, ?, ?) RETURNING id";
    let tag_id: i64 = tx.query_row(stmt, params![alias_id, "", category.id], |row| row.get(0))?;

    // Update tag alias with the new tag id
    stmt = "UPDATE tag_alias SET tagId = ? WHERE id = ?";
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
    let tag_result = find_by_name(conn, name)?;
    if let Some(tag) = tag_result {
        Ok(tag)
    } else {
        create(conn, name, None)
    }
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag_alias ta
        INNER JOIN tag t ON t.id = ta.tagId
        INNER JOIN tag_category tc ON t.categoryId = tc.id
        WHERE ta.name = ? AND t.primaryAliasId == ta.id")?;
    
    let tag_result = stmt.query_row(params![name], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            date_modified: row.get(3)?,
            category: row.get(4)?,
            aliases: vec![],
        })
    }).optional()?;

    if let Some(mut tag) = tag_result {
        let mut tag_alias_stmt = conn.prepare(
            "SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;
        let tag_alias_iter = tag_alias_stmt.query_map(params![&tag.id], |row| row.get(0))?;
        
        for alias in tag_alias_iter {
            tag.aliases.push(alias.unwrap());
        }

        Ok(Some(tag))
    } else {
        Ok(None)
    }
}

pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag t
        INNER JOIN tag_alias ta ON t.id = ta.tagId
        INNER JOIN tag_category tc ON t.categoryId = tc.id
        WHERE t.id = ? AND t.primaryAliasId == ta.id")?;
    
    let tag_result = stmt.query_row(params![id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            date_modified: row.get(3)?,
            category: row.get(4)?,
            aliases: vec![],
        })
    }).optional()?;

    if let Some(mut tag) = tag_result {
        let mut tag_alias_stmt = conn.prepare(
            "SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;
        let tag_alias_iter = tag_alias_stmt.query_map(params![&tag.id], |row| row.get(0))?;
        
        for alias in tag_alias_iter {
            tag.aliases.push(alias.unwrap());
        }

        Ok(Some(tag))
    } else {
        Ok(None)
    }
}

pub fn count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM tag", (), |row| row.get::<_, i64>(0))
}

pub fn delete(conn: &mut Connection, name: &str) -> Result<()> {
    let tag = find_by_name(conn, name)?;
    match tag {
        Some(tag) => {
            println!("Found tag");
            let games = crate::game::find_with_tag(conn, name)?;
            println!("{} games", games.len());

            let tx = conn.transaction()?;

            // Remove tag from games
            for game in games {
                let new_tags = game.detailed_tags.unwrap().iter().filter(|t| t.name != name).map(|t| t.name.clone()).collect::<Vec<String>>();
                tx.execute("UPDATE game SET tagsStr = ? WHERE id = ?", params![new_tags.join("; "), game.id])?;
            }

            let mut stmt = "DELETE FROM game_tags_tag WHERE tagId = ?";
            tx.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM tag_alias WHERE tagId = ?";
            tx.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM tag WHERE id = ?";
            tx.execute(stmt, params![tag.id])?;

            tx.commit()?;

            Ok(())
        },
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

pub fn merge_tag(conn: &mut Connection, name: &str, merged_into: &str) -> Result<Tag> {
    let old_tag = match find_by_name(conn, name)? {
        Some(tag) => tag,
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    };
    let merged_tag = match find_by_name(conn, merged_into)? {
        Some(tag) => tag,
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    };

    let tx = conn.transaction()?;

    // Remove future duplicate relations, add relations for all games with the old tag
    let mut stmt = "DELETE FROM game_tags_tag
    WHERE gameId IN (
        SELECT gameId FROM game_tags_tag WHERE tagId = ?
    )
    AND tagId = ?";
    tx.execute(stmt, params![old_tag.id, merged_tag.id])?;

    stmt = "UPDATE game_tags_tag SET tagId = ? WHERE tagId = ?";
    tx.execute(stmt, params![merged_tag.id, old_tag.id])?;

    // Remove old tag table entries
    stmt = "DELETE FROM tag WHERE id = ?";
    tx.execute(stmt, params![old_tag.id])?;
    stmt = "DELETE FROM tag_alias WHERE tagId = ?";
    tx.execute(stmt, params![old_tag.id])?;

    // Add aliases to new tag
    for alias in old_tag.aliases {
        stmt = "INSERT INTO tag_alias (tagId, name) VALUES (?, ?)";
        tx.execute(stmt, params![merged_tag.id, alias])?;
    }

    // Update game tagsStr
    stmt = "UPDATE game 
    SET tagsStr = (
      SELECT IFNULL(tags, '') tags FROM (
        SELECT GROUP_CONCAT(
          (SELECT name FROM tag_alias WHERE tagId = t.tagId), '; '
        ) tags
        FROM game_tags_tag t
        WHERE t.gameId = game.id
      )
    )";
    tx.execute(stmt, ())?;

    tx.commit()?;

    match find_by_name(conn, merged_into)? {
        Some(tag) => Ok(tag),
        None => Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn save(conn: &mut Connection, partial: &PartialTag) -> Result<Tag> {
    let mut tag = match find_by_id(conn, partial.id)? {
        Some(t) => t,
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    };

    let mut new_tag_aliases = vec![];

    tag.apply_partial(partial);

    // Check for collisions before updating
    for alias in tag.aliases.clone() {
        match find_by_name(conn, &alias)? {
            Some(existing_link) => {
                if existing_link.id != tag.id {
                    // Clash of alias, can't move until deassigned from other tag
                    return Err(rusqlite::Error::QueryReturnedNoRows) // TODO: Make this a proper error
                }
            },
            None => new_tag_aliases.push(alias),
        }
    }

    let tx = conn.transaction()?;


    // Apply flat edits
    match tag.category {
        Some(category) => {
            let stmt = "UPDATE tag SET description = ?, dateModified = ? category = (SELECT id FROM tag_category WHERE name = ?) WHERE id = ?";
            tx.execute(stmt, params![tag.description, tag.date_modified, category, tag.id])?;
        }
        None => {
            let stmt = "UPDATE tag SET description = ?, dateModified = ? WHERE id = ?";
            tx.execute(stmt, params![tag.description, tag.date_modified, tag.id])?;
        }
    }

    // Remove old aliases
    let mut stmt = "DELETE FROM tag_alias WHERE tagId = ? AND name NOT IN rarray(?)";
    let alias_rc = Rc::new(tag.aliases.iter().map(|v| Value::from(v.clone())).collect::<Vec<Value>>());
    tx.execute(stmt, params![tag.id, alias_rc])?;

    // Add new aliases
    for alias in new_tag_aliases {
        stmt = "INSERT INTO tag_alias (name, tagId) VALUES (?, ?)";
        tx.execute(stmt, params![alias, tag.id])?;
    }

    // Update primary alias id
    stmt = "UPDATE tag SET primaryAliasId = (SELECT id FROM tag_alias WHERE name = ?) WHERE id = ?";
    tx.execute(stmt, params![tag.name, tag.id])?;

    tx.commit()?;

    match find_by_id(&conn, tag.id)? {
        Some(t) => Ok(t),
        None => return Err(rusqlite::Error::QueryReturnedNoRows)
    }
}