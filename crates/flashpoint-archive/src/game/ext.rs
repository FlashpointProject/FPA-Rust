use std::collections::HashMap;

use crate::error::{self, Result};
use rusqlite::Connection;
use snafu::ResultExt;

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct ExtensionIndex {
    pub name: String,
    pub key: String,
}

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(not(feature = "napi"), derive(Clone))]
#[derive(PartialEq)]
pub enum ExtSearchableType {
    String,
    Boolean,
    Number,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct ExtSearchable {
    pub key: String,
    pub value_type: ExtSearchableType,
    pub search_key: String,
}

pub struct ExtSearchableRegistered {
    pub ext_id: String,
    pub key: String,
    pub value_type: ExtSearchableType,
}

#[derive(Clone)]
#[cfg_attr(feature = "napi", napi(object))]
pub struct ExtensionInfo {
    pub id: String,
    pub searchables: Vec<ExtSearchable>,
    pub indexes: Vec<ExtensionIndex>,
}

pub struct ExtensionRegistry {
    extensions: HashMap<String, ExtensionInfo>,
    pub searchables: HashMap<String, ExtSearchableRegistered>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        ExtensionRegistry {
            extensions: HashMap::new(),
            searchables: HashMap::new(),
        }
    }
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        ExtensionRegistry::default()
    }

    pub fn create_ext_indices(&self, conn: &Connection, ext: ExtensionInfo) -> Result<()>  {
        // Create relevant indices if missing
        self.create_indexes(conn, &ext)
        .context(error::SqliteSnafu)?;

        Ok(())
    }

    pub fn register_ext(&mut self, ext: ExtensionInfo) {
        // Insert to registry
        for searchable in &ext.searchables {
            self.searchables.insert(searchable.search_key.clone(), ExtSearchableRegistered {
                ext_id: ext.id.clone(),
                key: searchable.key.clone(),
                value_type: searchable.value_type.clone()
            });
        }
        self.extensions.insert(ext.id.clone(), ext);
    }

    fn create_indexes(&self, conn: &Connection, ext: &ExtensionInfo) -> rusqlite::Result<()> {
        // Create each new index
        for index in &ext.indexes {
            let index_name = format!("idx_ext_{}_{}", ext.id, index.name);

            let stmt = format!(
                "CREATE INDEX IF NOT EXISTS {} on ext_data(extId, JSON_EXTRACT(data, '$.{}'))",
                index_name, index.key
            );

            conn.execute(&stmt, [])?;
        }

        // TODO: Remove unused indicies

        Ok(())
    }
}
