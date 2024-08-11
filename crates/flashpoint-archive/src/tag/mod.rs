use std::rc::Rc;

use rusqlite::{params, types::Value, Connection, OptionalExtension, Result};

use crate::{
    game::search::{mark_index_dirty, SearchParam},
    tag_category, update::SqlVec,
};

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub date_modified: String,
    pub aliases: Vec<String>,
    pub category: Option<String>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct PartialTag {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub date_modified: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub category: Option<String>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct TagSuggestion {
    pub id: i64,
    pub name: String,
    pub matched_from: String,
    pub games_count: i64,
    pub category: Option<String>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone)]
pub struct LooseTagAlias {
    pub id: i64,
    pub value: String,
}

impl Tag {
    pub fn apply_partial(&mut self, partial: &PartialTag) {
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

        if let Some(date_modified) = partial.date_modified.clone() {
            self.date_modified = date_modified;
        }

        if let Some(category) = partial.category.clone() {
            self.category = Some(category);
        }
    }
}

impl Default for PartialTag {
    fn default() -> Self {
        return PartialTag {
            id: -1,
            name: String::new(),
            description: None,
            date_modified: None,
            aliases: None,
            category: None,
        };
    }
}

impl From<Tag> for PartialTag {
    fn from(value: Tag) -> Self {
        let mut partial = PartialTag::default();
        partial.id = value.id;
        partial.name = value.name;
        partial.description = Some(value.description);
        partial.date_modified = Some(value.date_modified);
        partial.aliases = Some(value.aliases);
        partial.category = value.category;
        partial
    }
}

pub fn find(conn: &Connection, tag_filter: Vec<String>) -> Result<Vec<Tag>> {
    let mut query = "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag t
                INNER JOIN tag_alias ta ON ta.id = t.primaryAliasId
                INNER JOIN tag_category tc ON t.categoryId = tc.id
                ORDER BY tc.name, ta.name";
    let mut params: Vec<SearchParam> = vec![];

    if tag_filter.len() > 0 {
        // Allow use of rarray() in SQL queries
        rusqlite::vtab::array::load_module(conn)?;
        query = "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag t
                INNER JOIN tag_alias ta ON ta.id = t.primaryAliasId
                INNER JOIN tag_category tc ON t.categoryId = tc.id
                WHERE t.id NOT IN (
                    SELECT tagId FROM tag_alias WHERE name IN rarray(?)
                )
                ORDER BY tc.name, ta.name";
        params.push(SearchParam::StringVec(tag_filter));
    }
    let mut stmt = conn.prepare(query)?;
    let params_as_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let tag_iter = stmt.query_map(params_as_refs.as_slice(), |row| {
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
        let mut tag_alias_stmt =
            conn.prepare("SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;
        let tag_alias_iter = tag_alias_stmt.query_map(params![&tag.id], |row| row.get(0))?;

        for alias in tag_alias_iter {
            tag.aliases.push(alias.unwrap());
        }
        tags.push(tag);
    }

    Ok(tags)
}

pub fn create(
    conn: &Connection,
    name: &str,
    category: Option<String>,
    id: Option<i64>,
) -> Result<Tag> {
    // Create the alias
    let mut stmt = "INSERT INTO tag_alias (name, tagId) VALUES(?, ?) RETURNING id";
    let category = tag_category::find_or_create(
        conn,
        category.unwrap_or_else(|| "default".to_owned()).as_str(),
        None,
    )?;

    // Create a new tag
    let alias_id: i64 = conn.query_row(stmt, params![name, -1], |row| row.get(0))?;

    match id {
        Some(id) => {
            stmt =
                "INSERT INTO tag (id, primaryAliasId, description, categoryId) VALUES (?, ?, ?, ?)";
            conn.execute(stmt, params![id, alias_id, "", category.id])?;

            // Update tag alias with the new tag id
            stmt = "UPDATE tag_alias SET tagId = ? WHERE id = ?";
            conn.execute(stmt, params![id, alias_id])?;
        }
        None => {
            stmt = "INSERT INTO tag (primaryAliasId, description, categoryId) VALUES (?, ?, ?) RETURNING id";
            let tag_id: i64 =
                conn.query_row(stmt, params![alias_id, "", category.id], |row| row.get(0))?;

            // Update tag alias with the new tag id
            stmt = "UPDATE tag_alias SET tagId = ? WHERE id = ?";
            conn.execute(stmt, params![tag_id, alias_id])?;
        }
    }

    mark_index_dirty(conn)?;

    let new_tag_result = find_by_name(conn, name)?;
    if let Some(tag) = new_tag_result {
        Ok(tag)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub fn find_or_create(conn: &Connection, name: &str) -> Result<Tag> {
    let tag_result = find_by_name(conn, name)?;
    if let Some(tag) = tag_result {
        Ok(tag)
    } else {
        // Clear a lingering alias
        conn.execute("DELETE FROM tag_alias WHERE name = ?", params![name])?;
        create(conn, name, None, None)
    }
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, ta.name, t.description, t.dateModified, tc.name FROM tag t
        INNER JOIN tag_alias ta ON t.id = ta.tagId
        INNER JOIN tag_category tc ON t.categoryId = tc.id
        WHERE t.id IN (SELECT alias.tagId FROM tag_alias alias WHERE alias.name = ?)
		AND t.primaryAliasId = ta.id",
    )?;

    let tag_result = stmt
        .query_row(params![name], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                date_modified: row.get(3)?,
                category: row.get(4)?,
                aliases: vec![],
            })
        })
        .optional()?;

    if let Some(mut tag) = tag_result {
        let mut tag_alias_stmt =
            conn.prepare("SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;
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
        WHERE t.id = ? AND t.primaryAliasId == ta.id",
    )?;

    let tag_result = stmt
        .query_row(params![id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                date_modified: row.get(3)?,
                category: row.get(4)?,
                aliases: vec![],
            })
        })
        .optional()?;

    if let Some(mut tag) = tag_result {
        let mut tag_alias_stmt =
            conn.prepare("SELECT ta.name FROM tag_alias ta WHERE ta.tagId = ?")?;
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

pub fn delete(conn: &Connection, name: &str) -> Result<()> {
    let tag = find_by_name(conn, name)?;
    match tag {
        Some(tag) => {
            let mut stmt = "DELETE FROM tag_alias WHERE tagId = ?";
            conn.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM tag WHERE id = ?";
            conn.execute(stmt, params![tag.id])?;

            // Update game tagsStr
            stmt = "UPDATE game
            SET tagsStr = (
                SELECT IFNULL(string_agg(ta.name, '; '), '')
                FROM game_tags_tag gtt
                JOIN tag t ON gtt.tagId = t.id
                JOIN tag_alias ta ON t.primaryAliasId = ta.id
                WHERE gtt.gameId = game.id
            ) WHERE game.id IN (
                SELECT gameId FROM game_tags_tag WHERE tagId = ?   
            )";
            conn.execute(stmt, params![tag.id])?;

            stmt = "DELETE FROM game_tags_tag WHERE tagId = ?";
            conn.execute(stmt, params![tag.id])?;

            mark_index_dirty(conn)?;

            Ok(())
        }
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

pub fn delete_by_id(conn: &Connection, id: i64) -> Result<()> {
    let mut stmt = "DELETE FROM tag_alias WHERE tagId = ?";
    conn.execute(stmt, params![id])?;

    stmt = "DELETE FROM tag WHERE id = ?";
    conn.execute(stmt, params![id])?;

    // Update game tagsStr
    stmt = "UPDATE game
    SET tagsStr = (
        SELECT IFNULL(string_agg(ta.name, '; '), '')
        FROM game_tags_tag gtt
        JOIN tag t ON gtt.tagId = t.id
        JOIN tag_alias ta ON t.primaryAliasId = ta.id
        WHERE gtt.gameId = game.id
    ) WHERE game.id IN (
        SELECT gameId FROM game_tags_tag WHERE tagId = ?   
    )";
    conn.execute(stmt, params![id])?;

    stmt = "DELETE FROM game_tags_tag WHERE tagId = ?";
    conn.execute(stmt, params![id])?;

    mark_index_dirty(conn)?;

    Ok(())
}

pub fn merge_tag(conn: &Connection, name: &str, merged_into: &str) -> Result<Tag> {
    let old_tag = match find_by_name(conn, name)? {
        Some(tag) => tag,
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    };
    let merged_tag = match find_by_name(conn, merged_into)? {
        Some(tag) => tag,
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    };

    // Remove future duplicate relations, add relations for all games with the old tag
    let mut stmt = "DELETE FROM game_tags_tag
    WHERE gameId IN (
        SELECT gameId FROM game_tags_tag WHERE tagId = ?
    )
    AND tagId = ?";
    conn.execute(stmt, params![old_tag.id, merged_tag.id])?;

    stmt = "UPDATE game_tags_tag SET tagId = ? WHERE tagId = ?";
    conn.execute(stmt, params![merged_tag.id, old_tag.id])?;

    // Remove old tag table entries
    stmt = "DELETE FROM tag WHERE id = ?";
    conn.execute(stmt, params![old_tag.id])?;
    stmt = "DELETE FROM tag_alias WHERE tagId = ?";
    conn.execute(stmt, params![old_tag.id])?;

    // Add aliases to new tag
    for alias in old_tag.aliases {
        stmt = "INSERT INTO tag_alias (tagId, name) VALUES (?, ?)";
        conn.execute(stmt, params![merged_tag.id, alias])?;
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
    conn.execute(stmt, ())?;

    mark_index_dirty(conn)?;

    match find_by_name(conn, merged_into)? {
        Some(tag) => Ok(tag),
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

pub fn save(conn: &Connection, partial: &PartialTag) -> Result<Tag> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    let mut tag = match find_by_id(conn, partial.id)? {
        Some(t) => t,
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    };

    let mut new_tag_aliases = vec![];

    tag.apply_partial(partial);

    let mut stmt = conn.prepare("SELECT tagId FROM tag_alias WHERE name = ?")?;

    // Check for collisions before updating
    for alias in tag.aliases.clone() {
        let existing_tag_id = stmt
            .query_row(params![alias], |row| row.get::<_, i64>(0))
            .optional()?;
        match existing_tag_id {
            Some(id) => {
                if id != tag.id {
                    return Err(rusqlite::Error::QueryReturnedNoRows); // TODO: Make this a proper error
                }
            }
            None => {
                new_tag_aliases.push(alias);
            }
        }
    }

    // Apply flat edits
    match tag.category {
        Some(category) => {
            let stmt = "UPDATE tag SET description = ?, dateModified = ?, categoryId = (SELECT id FROM tag_category WHERE name = ?) WHERE id = ?";
            conn.execute(
                stmt,
                params![tag.description, tag.date_modified, category, tag.id],
            )?;
        }
        None => {
            let stmt = "UPDATE tag SET description = ?, dateModified = ? WHERE id = ?";
            conn.execute(stmt, params![tag.description, tag.date_modified, tag.id])?;
        }
    }

    // Remove old aliases
    let mut stmt = "DELETE FROM tag_alias WHERE tagId = ? AND name NOT IN rarray(?)";
    let alias_rc = Rc::new(
        tag.aliases
            .iter()
            .map(|v| Value::from(v.clone()))
            .collect::<Vec<Value>>(),
    );
    conn.execute(stmt, params![tag.id, alias_rc])?;

    // Add new aliases
    for alias in new_tag_aliases {
        stmt = "INSERT INTO tag_alias (name, tagId) VALUES (?, ?)";
        conn.execute(stmt, params![alias, tag.id])?;
    }

    // Update primary alias id
    stmt = "UPDATE tag SET primaryAliasId = (SELECT id FROM tag_alias WHERE name = ?) WHERE id = ?";
    conn.execute(stmt, params![tag.name, tag.id])?;

    // Update game tagsStr fields
    stmt = "UPDATE game
    SET tagsStr = (
        SELECT IFNULL(string_agg(ta.name, '; '), '')
        FROM game_tags_tag gtt
        JOIN tag t ON gtt.tagId = t.id
        JOIN tag_alias ta ON t.primaryAliasId = ta.id
        WHERE gtt.gameId = game.id
    ) WHERE game.id IN (
        SELECT gameId FROM game_tags_tag WHERE tagId = ?   
    )";
    conn.execute(stmt, params![tag.id])?;

    mark_index_dirty(conn)?;

    match find_by_id(&conn, tag.id)? {
        Some(t) => Ok(t),
        None => return Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

pub fn search_tag_suggestions(
    conn: &Connection,
    partial: &str,
    blacklist: Vec<String>,
) -> Result<Vec<TagSuggestion>> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    let blacklist = SqlVec(blacklist);

    let mut suggestions = vec![];

    let query = "SELECT sugg.tagId, sugg.matched_alias, count(game_tag.gameId) as gameCount, sugg.primary_alias, sugg.category FROM (
        SELECT 
			ta1.tagId as tagId,
			ta1.name AS matched_alias,
			ta2.name AS primary_alias,
            cat.name as category
		FROM 
			tag_alias ta1
		JOIN 
			tag t ON ta1.tagId = t.id
		JOIN 
	        tag_alias ta2 ON t.primaryAliasId = ta2.id
        JOIN 
            tag_category cat ON t.categoryId = cat.id
		WHERE 
			ta1.name LIKE ?
    ) sugg
    LEFT JOIN game_tags_tag game_tag ON game_tag.tagId = sugg.tagId
    WHERE sugg.tagId NOT IN (
        SELECT tagId FROM tag_alias WHERE name IN rarray(?)
    )
    GROUP BY sugg.matched_alias
    ORDER BY COUNT(game_tag.gameId) DESC, sugg.matched_alias ASC";

    let mut stmt = conn.prepare(&query)?;
    let mut likeable = String::from(partial);
    likeable.push_str("%");
    let results = stmt.query_map(params![&likeable, blacklist], |row| {
        Ok(TagSuggestion {
            id: row.get(0)?,
            matched_from: row.get(1)?,
            games_count: row.get(2)?,
            name: row.get(3)?,
            category: row.get(4)?,
        })
    })?;

    for sugg in results {
        suggestions.push(sugg?);
    }

    Ok(suggestions)
}
