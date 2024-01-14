use chrono::NaiveDateTime;
use rusqlite::{Connection, Result, params, OptionalExtension};

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub date_modified: NaiveDateTime,
    pub aliases: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PartialTag {
    pub name: String,
    pub description: Option<String>,
    pub date_modified: Option<NaiveDateTime>,
    pub aliases: Option<Vec<String>>,
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

pub fn find_or_create(conn: &mut Connection, name: &str) -> Result<Tag> {
    let tag_result = find_by_name(conn, name)?;
    if let Some(tag) = tag_result {
        Ok(tag)
    } else {
        let tx = conn.transaction()?;
        // Create the alias
        let mut stmt = "INSERT INTO tag_alias (name, tagId) VALUES(?, ?) RETURNING id";

        // Create a new tag
        let alias_id: i64 = tx.query_row(stmt, params![name, -1], |row| row.get(0))?;

        stmt = "INSERT INTO tag (primaryAliasId, description, categoryId) VALUES (?, '', 1) RETURNING id";
        let tag_id: i64 = tx.query_row(stmt, params![alias_id], |row| row.get(0))?;

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
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag_alias ta
        INNER JOIN tag t ON t.id = ta.tagId
        INNER JOIN tag_category tc ON t.categoryId = tc.id
        WHERE ta.name = ?")?;
    
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

pub fn count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM tag", (), |row| row.get::<_, i64>(0))
}
