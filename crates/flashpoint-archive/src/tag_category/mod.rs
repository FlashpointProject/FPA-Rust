use rusqlite::{Connection, Result, params, OptionalExtension};

#[derive(Debug, Clone)]
pub struct TagCategory {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PartialTagCategory {
    pub name: String,
    pub color: String,
    pub description: Option<String>
}

impl From<&PartialTagCategory> for TagCategory {
    fn from(value: &PartialTagCategory) -> Self {
        TagCategory {
            id: -1,
            name: value.name.clone(),
            color: value.color.clone(),
            description: value.description.clone()
        }
    }
}

pub fn find(conn: &Connection) -> Result<Vec<TagCategory>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, description FROM tag_category"
    )?;

    let tag_category_iter = stmt.query_map((), |row| {
        Ok(TagCategory{
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            description: row.get(3)?,
        })
    })?;

    let mut tag_cats = vec![];
    for tc in tag_category_iter {
        tag_cats.push(tc?);
    }
    Ok(tag_cats)
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<TagCategory>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, description FROM tag_category WHERE name = ?"
    )?;

    Ok(stmt.query_row(params![name], |row| {
        Ok(TagCategory {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            description: row.get(3)?,
        })
    }).optional()?)
}


pub fn find_or_create(conn: &Connection, name: &str) -> Result<TagCategory> {
    let tag_category_result = find_by_name(conn, name)?;

    match tag_category_result {
        Some(tc) => Ok(tc),
        None => {
            let new_tag_category = PartialTagCategory {
                name: name.to_owned(),
                color: "#FFFFFF".to_owned(),
                description: None,
            };

            Ok(create(conn, &new_tag_category)?)
        }
    }
}

pub fn create(conn: &Connection, partial: &PartialTagCategory) -> Result<TagCategory> {
    let mut new_tag_category: TagCategory = partial.into();
    let mut stmt = conn.prepare(
        "INSERT INTO tag_category (name, color, description) VALUES (?, ?, ?) RETURNING id"
    )?;
    new_tag_category.id = stmt.query_row(params![new_tag_category.name, new_tag_category.color, new_tag_category.description], |row| row.get(0))?;
    Ok(new_tag_category)
}