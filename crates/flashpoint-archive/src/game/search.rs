use std::{collections::HashMap, fmt::Display, rc::Rc, hash::Hash};

use fancy_regex::{Captures, Regex};
use rusqlite::{
    params,
    types::{ToSqlOutput, Value, ValueRef},
    Connection, OptionalExtension, Result, ToSql,
};

use crate::{debug_println, game::{ext::ExtSearchableType, get_game_add_apps}};

use super::{ext::ExtSearchableRegistered, find_ext_data, get_game_data, get_game_platforms, get_game_tags, Game};

#[derive(Debug, Clone)]
pub enum SearchParam {
    Boolean(bool),
    String(String),
    StringVec(Vec<String>),
    Integer64(i64),
    Float64(f64),
    Value(serde_json::Value),
}

#[derive(Debug, Clone)]
pub struct TagFilterInfo {
    pub key: String,
    pub dirty: bool,
}

impl ToSql for SearchParam {
    fn to_sql(&self) -> Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SearchParam::Boolean(b) => Ok(ToSqlOutput::from(b.clone())),
            SearchParam::String(s) => Ok(ToSqlOutput::from(s.as_str())),
            SearchParam::StringVec(m) => {
                let v: Rc<Vec<Value>> = Rc::new(
                    m.iter()
                        .map(|v| Value::from(v.clone()))
                        .collect::<Vec<Value>>(),
                );
                Ok(ToSqlOutput::Array(v))
            }
            SearchParam::Integer64(i) => Ok(ToSqlOutput::from(i.clone())),
            SearchParam::Float64(f) => Ok(ToSqlOutput::from(f.clone())),
            SearchParam::Value(v) => match v {
                serde_json::Value::Null => Ok(ToSqlOutput::Borrowed(ValueRef::Null)),
                serde_json::Value::Number(n) if n.is_i64() => Ok(ToSqlOutput::from(n.as_i64().unwrap())),
                serde_json::Value::Number(n) if n.is_f64() => Ok(ToSqlOutput::from(n.as_f64().unwrap())),
                _ => serde_json::to_string(v)
                    .map(ToSqlOutput::from)
                    .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into())),
            },
        }
    }
}

impl Display for SearchParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchParam::Boolean(b) => f.write_str(b.to_string().as_str()),
            SearchParam::String(s) => f.write_str(s),
            SearchParam::StringVec(m) => f.write_str(format!("{}", m.join("', '")).as_str()),
            SearchParam::Integer64(i) => f.write_str(i.to_string().as_str()),
            SearchParam::Float64(nf) => f.write_str(nf.to_string().as_str()),
            SearchParam::Value(v) => f.write_str(serde_json::to_string(v).unwrap_or_default().as_str()),
        }
    }
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameSearch {
    pub filter: GameFilter,
    pub load_relations: GameSearchRelations,
    pub custom_id_order: Option<Vec<String>>,
    pub order: GameSearchOrder,
    pub ext_order: Option<GameSearchOrderExt>,
    pub offset: Option<GameSearchOffset>,
    pub limit: i64,
    pub slim: bool,
    pub with_tag_filter: Option<Vec<String>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameSearchOffset {
    pub value: serde_json::Value,
    pub title: String, // Secondary sort always
    pub game_id: String,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameSearchOrder {
    pub column: GameSearchSortable,
    pub direction: GameSearchDirection,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameSearchOrderExt {
    pub ext_id: String,
    pub key: String,
    pub default: serde_json::Value,
}

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(not(feature = "napi"), derive(Clone))]
#[derive(Debug, PartialEq)]
pub enum GameSearchSortable {
    TITLE,
    DEVELOPER,
    PUBLISHER,
    SERIES,
    PLATFORM,
    DATEADDED,
    DATEMODIFIED,
    RELEASEDATE,
    LASTPLAYED,
    PLAYTIME,
    RANDOM,
    CUSTOM,
}

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(not(feature = "napi"), derive(Clone))]
#[derive(Debug)]
pub enum GameSearchDirection {
    ASC,
    DESC,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameSearchRelations {
    pub tags: bool,
    pub platforms: bool,
    pub game_data: bool,
    pub add_apps: bool,
    pub ext_data: bool,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct GameFilter {
    pub subfilters: Vec<GameFilter>,
    pub whitelist: FieldFilter,
    pub blacklist: FieldFilter,
    pub exact_whitelist: FieldFilter,
    pub exact_blacklist: FieldFilter,
    pub lower_than: SizeFilter,
    pub higher_than: SizeFilter,
    pub equal_to: SizeFilter,
    pub bool_comp: BoolFilter,
    pub match_any: bool,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct FieldFilter {
    pub id: Option<Vec<String>>,
    pub generic: Option<Vec<String>>,
    pub library: Option<Vec<String>>,
    pub title: Option<Vec<String>>,
    pub developer: Option<Vec<String>>,
    pub publisher: Option<Vec<String>>,
    pub series: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
    pub play_mode: Option<Vec<String>>,
    pub status: Option<Vec<String>>,
    pub notes: Option<Vec<String>>,
    pub source: Option<Vec<String>>,
    pub original_description: Option<Vec<String>>,
    pub language: Option<Vec<String>>,
    pub application_path: Option<Vec<String>>,
    pub launch_command: Option<Vec<String>>,
    pub ruffle_support: Option<Vec<String>>,
    pub ext: Option<HashMap<String, HashMap<String, Vec<String>>>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct BoolFilter {
    pub installed: Option<bool>,
    pub ext: Option<HashMap<String, HashMap<String, bool>>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct SizeFilter {
    pub tags: Option<i64>,
    pub platforms: Option<i64>,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub release_date: Option<String>,
    pub game_data: Option<i64>,
    pub add_apps: Option<i64>,
    pub playtime: Option<i64>,
    pub playcount: Option<i64>,
    pub last_played: Option<String>,
    pub ext: Option<HashMap<String, HashMap<String, i64>>>,
}

#[derive(Debug, Clone)]
struct ForcedGameFilter {
    pub whitelist: ForcedFieldFilter,
    pub blacklist: ForcedFieldFilter,
    pub exact_whitelist: ForcedFieldFilter,
    pub exact_blacklist: ForcedFieldFilter,
    pub lower_than: SizeFilter,
    pub higher_than: SizeFilter,
    pub equal_to: SizeFilter,
    pub bool_comp: BoolFilter,
}

#[derive(Debug, Clone)]
struct ForcedFieldFilter {
    pub id: Vec<String>,
    pub generic: Vec<String>,
    pub library: Vec<String>,
    pub title: Vec<String>,
    pub developer: Vec<String>,
    pub publisher: Vec<String>,
    pub series: Vec<String>,
    pub tags: Vec<String>,
    pub platforms: Vec<String>,
    pub play_mode: Vec<String>,
    pub status: Vec<String>,
    pub notes: Vec<String>,
    pub source: Vec<String>,
    pub original_description: Vec<String>,
    pub language: Vec<String>,
    pub application_path: Vec<String>,
    pub launch_command: Vec<String>,
    pub ruffle_support: Vec<String>,
    pub ext: HashMap<String, HashMap<String, Vec<String>>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct PageTuple {
    pub id: String,
    pub order_val: serde_json::Value,
    pub title: String,
}

impl Default for GameSearch {
    fn default() -> Self {
        GameSearch {
            filter: GameFilter::default(),
            load_relations: GameSearchRelations::default(),
            order: GameSearchOrder {
                column: GameSearchSortable::TITLE,
                direction: GameSearchDirection::ASC,
            },
            custom_id_order: None,
            ext_order: None,
            offset: None,
            limit: 1000,
            slim: false,
            with_tag_filter: None,
        }
    }
}

impl Default for GameFilter {
    fn default() -> Self {
        GameFilter {
            subfilters: vec![],
            whitelist: FieldFilter::default(),
            blacklist: FieldFilter::default(),
            exact_whitelist: FieldFilter::default(),
            exact_blacklist: FieldFilter::default(),
            lower_than: SizeFilter::default(),
            higher_than: SizeFilter::default(),
            equal_to: SizeFilter::default(),
            bool_comp: BoolFilter::default(),
            match_any: false,
        }
    }
}

impl Default for GameSearchRelations {
    fn default() -> Self {
        GameSearchRelations {
            tags: false,
            platforms: false,
            game_data: false,
            add_apps: false,
            ext_data: true,
        }
    }
}

impl Default for FieldFilter {
    fn default() -> Self {
        FieldFilter {
            id: None,
            generic: None,
            library: None,
            title: None,
            developer: None,
            publisher: None,
            series: None,
            tags: None,
            platforms: None,
            play_mode: None,
            status: None,
            notes: None,
            source: None,
            original_description: None,
            language: None,
            application_path: None,
            launch_command: None,
            ruffle_support: None,
            ext: None,
        }
    }
}

impl Default for ForcedGameFilter {
    fn default() -> Self {
        ForcedGameFilter {
            whitelist: ForcedFieldFilter::default(),
            blacklist: ForcedFieldFilter::default(),
            exact_whitelist: ForcedFieldFilter::default(),
            exact_blacklist: ForcedFieldFilter::default(),
            lower_than: SizeFilter::default(),
            higher_than: SizeFilter::default(),
            equal_to: SizeFilter::default(),
            bool_comp: BoolFilter::default(),
        }
    }
}

impl Default for ForcedFieldFilter {
    fn default() -> Self {
        ForcedFieldFilter {
            id: vec![],
            generic: vec![],
            library: vec![],
            title: vec![],
            developer: vec![],
            publisher: vec![],
            series: vec![],
            tags: vec![],
            platforms: vec![],
            play_mode: vec![],
            status: vec![],
            notes: vec![],
            source: vec![],
            original_description: vec![],
            language: vec![],
            application_path: vec![],
            launch_command: vec![],
            ruffle_support: vec![],
            ext: HashMap::default(),
        }
    }
}

impl Default for SizeFilter {
    fn default() -> Self {
        return SizeFilter {
            tags: None,
            platforms: None,
            date_added: None,
            date_modified: None,
            release_date: None,
            game_data: None,
            add_apps: None,
            playtime: None,
            playcount: None,
            last_played: None,
            ext: None,
        };
    }
}

impl Default for BoolFilter {
    fn default() -> Self {
        return BoolFilter {
            installed: None,
            ext: None,
        };
    }
}

impl From<&ForcedGameFilter> for GameFilter {
    fn from(value: &ForcedGameFilter) -> Self {
        let mut search = GameFilter::default();

        // Whitelist

        if value.whitelist.id.len() > 0 {
            search.whitelist.id = Some(value.whitelist.id.clone());
        }
        if value.whitelist.generic.len() > 0 {
            search.whitelist.generic = Some(value.whitelist.generic.clone());
        }
        if value.whitelist.title.len() > 0 {
            search.whitelist.title = Some(value.whitelist.title.clone());
        }
        if value.whitelist.developer.len() > 0 {
            search.whitelist.developer = Some(value.whitelist.developer.clone());
        }
        if value.whitelist.publisher.len() > 0 {
            search.whitelist.publisher = Some(value.whitelist.publisher.clone());
        }
        if value.whitelist.series.len() > 0 {
            search.whitelist.series = Some(value.whitelist.series.clone());
        }
        if value.whitelist.tags.len() > 0 {
            search.whitelist.tags = Some(value.whitelist.tags.clone());
        }
        if value.whitelist.platforms.len() > 0 {
            search.whitelist.platforms = Some(value.whitelist.platforms.clone());
        }
        if value.whitelist.play_mode.len() > 0 {
            search.whitelist.play_mode = Some(value.whitelist.play_mode.clone());
        }
        if value.whitelist.status.len() > 0 {
            search.whitelist.status = Some(value.whitelist.status.clone());
        }
        if value.whitelist.notes.len() > 0 {
            search.whitelist.notes = Some(value.whitelist.notes.clone());
        }
        if value.whitelist.source.len() > 0 {
            search.whitelist.source = Some(value.whitelist.source.clone());
        }
        if value.whitelist.original_description.len() > 0 {
            search.whitelist.original_description =
                Some(value.whitelist.original_description.clone());
        }
        if value.whitelist.language.len() > 0 {
            search.whitelist.language = Some(value.whitelist.language.clone());
        }
        if value.whitelist.application_path.len() > 0 {
            search.whitelist.application_path = Some(value.whitelist.application_path.clone());
        }
        if value.whitelist.launch_command.len() > 0 {
            search.whitelist.launch_command = Some(value.whitelist.launch_command.clone());
        }
        if value.whitelist.ruffle_support.len() > 0 {
            search.whitelist.ruffle_support = Some(value.whitelist.ruffle_support.clone());
        }
        if value.whitelist.ext.len() > 0 {
            search.whitelist.ext = Some(value.whitelist.ext.clone());
        }

        // Blacklist

        if value.blacklist.id.len() > 0 {
            search.blacklist.id = Some(value.blacklist.id.clone());
        }
        if value.blacklist.generic.len() > 0 {
            search.blacklist.generic = Some(value.blacklist.generic.clone());
        }
        if value.blacklist.title.len() > 0 {
            search.blacklist.title = Some(value.blacklist.title.clone());
        }
        if value.blacklist.developer.len() > 0 {
            search.blacklist.developer = Some(value.blacklist.developer.clone());
        }
        if value.blacklist.publisher.len() > 0 {
            search.blacklist.publisher = Some(value.blacklist.publisher.clone());
        }
        if value.blacklist.series.len() > 0 {
            search.blacklist.series = Some(value.blacklist.series.clone());
        }
        if value.blacklist.tags.len() > 0 {
            search.blacklist.tags = Some(value.blacklist.tags.clone());
        }
        if value.blacklist.platforms.len() > 0 {
            search.blacklist.platforms = Some(value.blacklist.platforms.clone());
        }
        if value.blacklist.play_mode.len() > 0 {
            search.blacklist.play_mode = Some(value.blacklist.play_mode.clone());
        }
        if value.blacklist.status.len() > 0 {
            search.blacklist.status = Some(value.blacklist.status.clone());
        }
        if value.blacklist.notes.len() > 0 {
            search.blacklist.notes = Some(value.blacklist.notes.clone());
        }
        if value.blacklist.source.len() > 0 {
            search.blacklist.source = Some(value.blacklist.source.clone());
        }
        if value.blacklist.original_description.len() > 0 {
            search.blacklist.original_description =
                Some(value.blacklist.original_description.clone());
        }
        if value.blacklist.language.len() > 0 {
            search.blacklist.language = Some(value.blacklist.language.clone());
        }
        if value.blacklist.application_path.len() > 0 {
            search.blacklist.application_path = Some(value.blacklist.application_path.clone());
        }
        if value.blacklist.launch_command.len() > 0 {
            search.blacklist.launch_command = Some(value.blacklist.launch_command.clone());
        }
        if value.blacklist.ruffle_support.len() > 0 {
            search.blacklist.ruffle_support = Some(value.blacklist.ruffle_support.clone());
        }
        if value.blacklist.ext.len() > 0 {
            search.blacklist.ext = Some(value.blacklist.ext.clone());
        }

        // Exact whitelist

        if value.exact_whitelist.id.len() > 0 {
            search.exact_whitelist.id = Some(value.exact_whitelist.id.clone());
        }
        if value.exact_whitelist.generic.len() > 0 {
            search.exact_whitelist.generic = Some(value.exact_whitelist.generic.clone());
        }
        if value.exact_whitelist.title.len() > 0 {
            search.exact_whitelist.title = Some(value.exact_whitelist.title.clone());
        }
        if value.exact_whitelist.developer.len() > 0 {
            search.exact_whitelist.developer = Some(value.exact_whitelist.developer.clone());
        }
        if value.exact_whitelist.publisher.len() > 0 {
            search.exact_whitelist.publisher = Some(value.exact_whitelist.publisher.clone());
        }
        if value.exact_whitelist.series.len() > 0 {
            search.exact_whitelist.series = Some(value.exact_whitelist.series.clone());
        }
        if value.exact_whitelist.tags.len() > 0 {
            search.exact_whitelist.tags = Some(value.exact_whitelist.tags.clone());
        }
        if value.exact_whitelist.platforms.len() > 0 {
            search.exact_whitelist.platforms = Some(value.exact_whitelist.platforms.clone());
        }
        if value.exact_whitelist.play_mode.len() > 0 {
            search.exact_whitelist.play_mode = Some(value.exact_whitelist.play_mode.clone());
        }
        if value.exact_whitelist.status.len() > 0 {
            search.exact_whitelist.status = Some(value.exact_whitelist.status.clone());
        }
        if value.exact_whitelist.notes.len() > 0 {
            search.exact_whitelist.notes = Some(value.exact_whitelist.notes.clone());
        }
        if value.exact_whitelist.source.len() > 0 {
            search.exact_whitelist.source = Some(value.exact_whitelist.source.clone());
        }
        if value.exact_whitelist.original_description.len() > 0 {
            search.exact_whitelist.original_description =
                Some(value.exact_whitelist.original_description.clone());
        }
        if value.exact_whitelist.language.len() > 0 {
            search.exact_whitelist.language = Some(value.exact_whitelist.language.clone());
        }
        if value.exact_whitelist.application_path.len() > 0 {
            search.exact_whitelist.application_path =
                Some(value.exact_whitelist.application_path.clone());
        }
        if value.exact_whitelist.launch_command.len() > 0 {
            search.exact_whitelist.launch_command =
                Some(value.exact_whitelist.launch_command.clone());
        }
        if value.exact_whitelist.ruffle_support.len() > 0 {
            search.exact_whitelist.ruffle_support =
                Some(value.exact_whitelist.ruffle_support.clone());
        }
        if value.exact_whitelist.ext.len() > 0 {
            search.exact_whitelist.ext = Some(value.exact_whitelist.ext.clone());
        }

        // Exact blacklist

        if value.exact_blacklist.id.len() > 0 {
            search.exact_blacklist.id = Some(value.exact_blacklist.id.clone());
        }
        if value.exact_blacklist.generic.len() > 0 {
            search.exact_blacklist.generic = Some(value.exact_blacklist.generic.clone());
        }
        if value.exact_blacklist.title.len() > 0 {
            search.exact_blacklist.title = Some(value.exact_blacklist.title.clone());
        }
        if value.exact_blacklist.developer.len() > 0 {
            search.exact_blacklist.developer = Some(value.exact_blacklist.developer.clone());
        }
        if value.exact_blacklist.publisher.len() > 0 {
            search.exact_blacklist.publisher = Some(value.exact_blacklist.publisher.clone());
        }
        if value.exact_blacklist.series.len() > 0 {
            search.exact_blacklist.series = Some(value.exact_blacklist.series.clone());
        }
        if value.exact_blacklist.tags.len() > 0 {
            search.exact_blacklist.tags = Some(value.exact_blacklist.tags.clone());
        }
        if value.exact_blacklist.platforms.len() > 0 {
            search.exact_blacklist.platforms = Some(value.exact_blacklist.platforms.clone());
        }
        if value.exact_blacklist.play_mode.len() > 0 {
            search.exact_blacklist.play_mode = Some(value.exact_blacklist.play_mode.clone());
        }
        if value.exact_blacklist.status.len() > 0 {
            search.exact_blacklist.status = Some(value.exact_blacklist.status.clone());
        }
        if value.exact_blacklist.notes.len() > 0 {
            search.exact_blacklist.notes = Some(value.exact_blacklist.notes.clone());
        }
        if value.exact_blacklist.source.len() > 0 {
            search.exact_blacklist.source = Some(value.exact_blacklist.source.clone());
        }
        if value.exact_blacklist.original_description.len() > 0 {
            search.exact_blacklist.original_description =
                Some(value.exact_blacklist.original_description.clone());
        }
        if value.exact_blacklist.language.len() > 0 {
            search.exact_blacklist.language = Some(value.exact_blacklist.language.clone());
        }
        if value.exact_blacklist.application_path.len() > 0 {
            search.exact_blacklist.application_path =
                Some(value.exact_blacklist.application_path.clone());
        }
        if value.exact_blacklist.launch_command.len() > 0 {
            search.exact_blacklist.launch_command =
                Some(value.exact_blacklist.launch_command.clone());
        }
        if value.exact_blacklist.ruffle_support.len() > 0 {
            search.exact_blacklist.ruffle_support =
                Some(value.exact_blacklist.ruffle_support.clone());
        }
        if value.exact_blacklist.ext.len() > 0 {
            search.exact_blacklist.ext = Some(value.exact_blacklist.ext.clone());
        }

        search.higher_than = value.higher_than.clone();
        search.lower_than = value.lower_than.clone();
        search.equal_to = value.equal_to.clone();
        search.bool_comp = value.bool_comp.clone();

        search
    }
}

pub trait InsertOrGet<K: Eq + Hash, V: Default> {
    fn insert_or_get(&mut self, item: K) -> &mut V;
}

impl<K: Eq + Hash, V: Default> InsertOrGet<K, V> for HashMap<K, V> {
    fn insert_or_get(&mut self, item: K) -> &mut V {
        return match self.entry(item) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => v.insert(V::default()),
        };
    }
}

macro_rules! whitelist_clause {
    ($func:ident, $field_name:expr, $filter:expr) => {
        $func($field_name, $filter, false, false)
    };
}

macro_rules! blacklist_clause {
    ($func:ident, $field_name:expr, $filter:expr) => {
        $func($field_name, $filter, false, true)
    };
}

macro_rules! exact_whitelist_clause {
    ($func:ident, $field_name:expr, $filter:expr) => {
        $func($field_name, $filter, true, false)
    };
}

macro_rules! exact_blacklist_clause {
    ($func:ident, $field_name:expr, $filter:expr) => {
        $func($field_name, $filter, true, true)
    };
}

const COUNT_QUERY: &str = "SELECT COUNT(*) FROM game";

const RESULTS_QUERY: &str =
    "SELECT game.id, title, alternateTitles, series, developer, publisher, platformsStr, \
platformName, dateAdded, dateModified, broken, extreme, playMode, status, notes, \
tagsStr, source, applicationPath, launchCommand, releaseDate, version, \
originalDescription, language, activeDataId, activeDataOnDisk, lastPlayed, playtime, \
activeGameConfigId, activeGameConfigOwner, archiveState, library, playCounter, logoPath, screenshotPath, ruffleSupport \
FROM game";

const SLIM_RESULTS_QUERY: &str =
    "SELECT game.id, title, series, developer, publisher, platformsStr, 
platformName, tagsStr, library, logoPath, screenshotPath 
FROM game";

const TAG_FILTER_INDEX_QUERY: &str = "INSERT INTO tag_filter_index (id) SELECT game.id FROM game";

pub fn search_index(
    conn: &Connection,
    search: &mut GameSearch,
    limit: Option<i64>,
) -> Result<Vec<PageTuple>> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    // Update tag filter indexing
    if let Some(tags) = &search.with_tag_filter {
        if tags.len() > 0 {
            let mut filtered_search = GameSearch::default();
            filtered_search.limit = 999999999;
            filtered_search.filter.exact_blacklist.tags = Some(tags.to_vec());
            filtered_search.filter.match_any = true;
            new_tag_filter_index(conn, &mut filtered_search)?;
        }
    }

    if search.order.column == GameSearchSortable::CUSTOM {
        if let Some(custom_id_order) = &search.custom_id_order {
            if custom_id_order.len() > 0 {
                new_custom_id_order(conn, custom_id_order.clone())?;
            }
        }
    }

    let order_column = match search.order.column {
        GameSearchSortable::TITLE => "game.title",
        GameSearchSortable::DEVELOPER => "game.developer",
        GameSearchSortable::PUBLISHER => "game.publisher",
        GameSearchSortable::SERIES => "game.series",
        GameSearchSortable::PLATFORM => "game.platformName",
        GameSearchSortable::DATEADDED => "game.dateAdded",
        GameSearchSortable::DATEMODIFIED => "game.dateModified",
        GameSearchSortable::RELEASEDATE => "game.releaseDate",
        GameSearchSortable::LASTPLAYED => "game.lastPlayed",
        GameSearchSortable::PLAYTIME => "game.playtime",
        GameSearchSortable::CUSTOM => "RowNum",
        _ => "unknown",
    };
    let order_direction = match search.order.direction {
        GameSearchDirection::ASC => "ASC",
        GameSearchDirection::DESC => "DESC",
    };
    let page_size = search.limit;
    search.limit = limit.or_else(|| Some(999999999)).unwrap();
    let selection = match &search.ext_order {
        Some(ext_order) => format!("
            WITH OrderedExt AS (
                SELECT
                    gameId AS id,
                    COALESCE(JSON_EXTRACT(data, '$.{}'), {}) AS ExtValue
                FROM ext_data
                WHERE extId = '{}'
            )
            SELECT 
                game.id, 
                OrderedExt.ExtValue, 
                game.title, 
                ROW_NUMBER() OVER (ORDER BY OrderedExt.ExtValue, game.title, game.id) AS rn 
            FROM game", 
            ext_order.key, ext_order.default.to_string(), ext_order.ext_id),
        None => match search.order.column {
            GameSearchSortable::CUSTOM => "
            WITH OrderedIDs AS (
                SELECT
                id,
                ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS RowNum
                FROM custom_id_order
            ) 
            SELECT game.id, OrderedIDs.RowNum, game.title, ROW_NUMBER() OVER (ORDER BY OrderedIDs.RowNum, game.title, game.id) AS rn FROM game".to_owned(),
            _ => format!("SELECT game.id, {}, game.title, ROW_NUMBER() OVER (ORDER BY {} COLLATE NOCASE {}, game.title {}, game.id) AS rn FROM game", order_column, order_column, order_direction, order_direction)
        }
    };

    // Override ordering for ext sorts
    let adjusted_order_column = match &search.ext_order {
        Some(_) => "ExtValue",
        None => order_column
    };

    let (mut query, mut params) = build_search_query(search, &selection);
    
    // Add the weirdness
    query = format!(
        "SELECT game.id, {}, game.title FROM ({}) game WHERE rn % ? = 0",
        adjusted_order_column, query
    );
    params.push(SearchParam::String(page_size.to_string()));

    let params_as_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let mut keyset = vec![];
    debug_println!(
        "search index query - \n{}",
        format_query(&query, params.clone())
    );
    let mut stmt = conn.prepare(&query)?;
    let page_tuple_iter = stmt.query_map(params_as_refs.as_slice(), |row| {
        let order_val = match row.get::<_, Option<Value>>(1)? {
            Some(value) => value,
            None => Value::Text("".to_string()), // Handle NULL as you see fit
        };
        Ok(PageTuple {
            id: row.get(0)?,
            order_val: match order_val {
                Value::Text(v) => serde_json::Value::String(v),
                Value::Integer(v) => serde_json::Value::Number(v.into()),
                Value::Real(v) => serde_json::Value::Number(
                    serde_json::Number::from_f64(v).unwrap_or_else(|| serde_json::Number::from(0))
                ),
                _ => serde_json::Value::Null
            },
            title: row.get(2)?,
        })
    })?;
    for page_tuple in page_tuple_iter {
        keyset.push(page_tuple?);
    }
    Ok(keyset)
}

pub fn search_count(conn: &Connection, search: &GameSearch) -> Result<i64> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    let mut selection = COUNT_QUERY.to_owned();
    if let Some(ext_order) = &search.ext_order {
        selection = format!("WITH OrderedExt AS (
            SELECT
                gameId AS id,
                COALESCE(JSON_EXTRACT(data, '$.{}'), {}) AS ExtValue
            FROM ext_data
            WHERE extId = '{}'
        ) ", ext_order.key, ext_order.default.to_string(), ext_order.ext_id)
            + &selection;
    } else if search.order.column == GameSearchSortable::CUSTOM {
        selection = "WITH OrderedIDs AS (
            SELECT
            id,
            ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS RowNum
            FROM custom_id_order
        ) "
        .to_owned()
            + &selection;
    }
    
    let (query, params) = build_search_query(search, &selection);
    debug_println!(
        "search count query - \n{}",
        format_query(&query, params.clone())
    );

    let params_as_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let count_result = conn
        .query_row(&query, params_as_refs.as_slice(), |row| {
            row.get::<_, i64>(0)
        })
        .optional()?;

    match count_result {
        Some(count) => Ok(count),
        None => Ok(0),
    }
}

pub fn search_custom<T, F>(
    conn: &Connection,
    search: &GameSearch,
    selection: &str,
    game_map_closure: F,
) -> Result<Vec<T>>
where
    F: Fn(&rusqlite::Row<'_>) -> Result<T>,
{
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    let (query, params) = build_search_query(search, selection);
    debug_println!("search query - \n{}", format_query(&query, params.clone()));

    // Convert the parameters array to something rusqlite understands
    let params_as_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let mut results = Vec::new();

    let mut stmt = conn.prepare(query.as_str())?;
    let row_iter = stmt.query_map(params_as_refs.as_slice(), game_map_closure)?;

    for result in row_iter {
        results.push(result?);
    }

    Ok(results)
}

// The search function that takes a connection and a GameSearch object
pub fn search(conn: &Connection, search: &GameSearch) -> Result<Vec<Game>> {
    let mut selection = match search.slim {
        true => SLIM_RESULTS_QUERY.to_owned(),
        false => RESULTS_QUERY.to_owned(),
    };
    if let Some(ext_order) = &search.ext_order {
        selection = format!("WITH OrderedExt AS (
            SELECT
                gameId AS id,
                COALESCE(JSON_EXTRACT(data, '$.{}'), {}) AS ExtValue
            FROM ext_data
            WHERE extId = '{}'
        ) ", ext_order.key, ext_order.default.to_string(), ext_order.ext_id)
            + &selection;
    } else if search.order.column == GameSearchSortable::CUSTOM {
        selection = "WITH OrderedIDs AS (
            SELECT
            id,
            ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS RowNum
            FROM custom_id_order
        ) "
        .to_owned()
            + &selection;
    }

    let game_map_closure = match search.slim {
        true => |row: &rusqlite::Row<'_>| -> Result<Game> {
            Ok(Game {
                id: row.get(0)?,
                title: row.get(1)?,
                series: row.get(2)?,
                developer: row.get(3)?,
                publisher: row.get(4)?,
                platforms: row.get(5)?,
                primary_platform: row.get(6)?,
                tags: row.get(7)?,
                library: row.get(8)?,
                logo_path: row.get(9)?,
                screenshot_path: row.get(10)?,
                ..Default::default()
            })
        },
        false => |row: &rusqlite::Row<'_>| -> Result<Game> {
            Ok(Game {
                id: row.get(0)?,
                title: row.get(1)?,
                alternate_titles: row.get(2)?,
                series: row.get(3)?,
                developer: row.get(4)?,
                publisher: row.get(5)?,
                platforms: row.get(6)?,
                primary_platform: row.get(7)?,
                date_added: row.get(8)?,
                date_modified: row.get(9)?,
                legacy_broken: row.get(10)?,
                legacy_extreme: row.get(11)?,
                play_mode: row.get(12)?,
                status: row.get(13)?,
                notes: row.get(14)?,
                tags: row.get(15)?,
                source: row.get(16)?,
                legacy_application_path: row.get(17)?,
                legacy_launch_command: row.get(18)?,
                release_date: row.get(19)?,
                version: row.get(20)?,
                original_description: row.get(21)?,
                language: row.get(22)?,
                active_data_id: row.get(23)?,
                active_data_on_disk: row.get(24)?,
                last_played: row.get(25)?,
                playtime: row.get(26)?,
                active_game_config_id: row.get(27)?,
                active_game_config_owner: row.get(28)?,
                archive_state: row.get(29)?,
                library: row.get(30)?,
                play_counter: row.get(31)?,
                detailed_platforms: None,
                detailed_tags: None,
                game_data: None,
                add_apps: None,
                logo_path: row.get(32)?,
                screenshot_path: row.get(33)?,
                ruffle_support: row.get(34)?,
                ext_data: None,
            })
        },
    };

    let mut games = search_custom(conn, search, selection.as_str(), game_map_closure)?;

    for game in &mut games {
        if search.load_relations.platforms {
            game.detailed_platforms = get_game_platforms(conn, &game.id)?.into();
        }
        if search.load_relations.tags {
            game.detailed_tags = get_game_tags(conn, &game.id)?.into();
        }
        if search.load_relations.game_data {
            game.game_data = Some(get_game_data(conn, &game.id)?);
        }
        if search.load_relations.add_apps {
            game.add_apps = Some(get_game_add_apps(conn, &game.id)?);
        }
        if search.load_relations.ext_data {
            game.ext_data = Some(find_ext_data(conn, &game.id)?);
        }
    }

    Ok(games)
}

pub fn search_random(conn: &Connection, mut s: GameSearch, count: i64) -> Result<Vec<Game>> {
    s.limit = count;
    s.order.column = GameSearchSortable::RANDOM;

    // Update tag filter indexing
    if let Some(tags) = &s.with_tag_filter {
        if tags.len() > 0 {
            let mut filtered_search = GameSearch::default();
            filtered_search.limit = 999999999;
            filtered_search.filter.exact_blacklist.tags = Some(tags.to_vec());
            filtered_search.filter.match_any = true;
            new_tag_filter_index(conn, &mut filtered_search)?;
        }
    }

    search(conn, &s)
}

fn build_search_query(search: &GameSearch, selection: &str) -> (String, Vec<SearchParam>) {
    let mut query = String::from(selection);

    if search.ext_order.is_some() {
        query.push_str(" INNER JOIN OrderedExt ON game.id = OrderedExt.id");
    } else if search.order.column == GameSearchSortable::CUSTOM {
        query.push_str(" INNER JOIN OrderedIDs ON game.id = OrderedIDs.id");
    }

    // Ordering
    let order_column = match search.ext_order {
        Some(_) => "OrderedExt.ExtValue",
        None => match search.order.column {
            GameSearchSortable::TITLE => "game.title",
            GameSearchSortable::DEVELOPER => "game.developer",
            GameSearchSortable::PUBLISHER => "game.publisher",
            GameSearchSortable::SERIES => "game.series",
            GameSearchSortable::PLATFORM => "game.platformName",
            GameSearchSortable::DATEADDED => "game.dateAdded",
            GameSearchSortable::DATEMODIFIED => "game.dateModified",
            GameSearchSortable::RELEASEDATE => "game.releaseDate",
            GameSearchSortable::LASTPLAYED => "game.lastPlayed",
            GameSearchSortable::PLAYTIME => "game.playtime",
            GameSearchSortable::CUSTOM => "OrderedIDs.RowNum",
            _ => "unknown",
        }
    };
    let order_direction = match search.order.direction {
        GameSearchDirection::ASC => "ASC",
        GameSearchDirection::DESC => "DESC",
    };

    // Build the inner WHERE clause
    let mut params: Vec<SearchParam> = vec![];
    let where_clause = build_filter_query(&search.filter, &mut params);

    // Add tag filtering
    if let Some(tags) = &search.with_tag_filter {
        if tags.len() > 0 {
            query.push_str(" INNER JOIN tag_filter_index ON game.id = tag_filter_index.id");
        }
    }

    // Add offset
    if let Some(offset) = search.offset.clone() {
        let offset_val = match offset.value {
            serde_json::Value::Number(number) => SearchParam::Float64(number.as_f64().unwrap_or(0.into())),
            val => SearchParam::String(val.as_str().unwrap_or("").to_owned()),
        };
        if search.order.column == GameSearchSortable::CUSTOM {
            let offset_clause = format!(" WHERE OrderedIDs.RowNum > ?");
            query.push_str(&offset_clause);
            params.insert(0, offset_val);
        } else {
            let offset_clause = match search.order.direction {
                GameSearchDirection::ASC => {
                    format!(
                        " WHERE ({} COLLATE NOCASE, game.title, game.id) > (?, ?, ?)",
                        order_column
                    )
                }
                GameSearchDirection::DESC => {
                    format!(
                        " WHERE ({} COLLATE NOCASE, game.title, game.id) < (?, ?, ?)",
                        order_column
                    )
                }
            };
            query.push_str(&offset_clause);

            // Insert in reverse order
            params.insert(0, SearchParam::String(offset.game_id.clone()));
            params.insert(0, SearchParam::String(offset.title.clone()));
            params.insert(0, offset_val);
        }
    }

    // Combine all where clauses
    if where_clause.len() > 0 && where_clause != "()" {
        // Offset will begin WHERE itself, otherwise we're ANDing the offset
        let start_clause = match search.offset {
            Some(_) => " AND (",
            None => " WHERE (",
        };
        query.push_str(start_clause);
        query.push_str(&where_clause);
        query.push_str(")");
    }

    if search.order.column == GameSearchSortable::RANDOM {
        query.push_str(" ORDER BY RANDOM()");
        let limit_query = format!(" LIMIT {}", search.limit);
        query.push_str(&limit_query);
    } else {
        if search.order.column == GameSearchSortable::CUSTOM {
            query.push_str(" ORDER BY OrderedIDs.RowNum");
        } else {
            query.push_str(
                format!(
                    " ORDER BY {} COLLATE NOCASE {}, game.title {}",
                    order_column, order_direction, order_direction
                )
                .as_str(),
            );
        }
        let limit_query = format!(" LIMIT {}", search.limit);
        query.push_str(&limit_query);
    }

    (query, params)
}

fn build_filter_query(filter: &GameFilter, params: &mut Vec<SearchParam>) -> String {
    let mut where_clauses = Vec::new();

    if filter.subfilters.len() > 0 {
        for subfilter in filter.subfilters.iter() {
            let new_clause = build_filter_query(subfilter, params);
            if new_clause != "" {
                where_clauses.push(format!("({})", new_clause));
            }
        }
    }

    let mut add_clause =
        |field_name: &str, values: &Option<Vec<String>>, exact: bool, blacklist: bool| {
            if let Some(value_list) = values {
                let comparator = match (blacklist, exact) {
                    (true, true) => "!=",
                    (true, false) => "NOT LIKE",
                    (false, true) => "=",
                    (false, false) => "LIKE",
                };

                // Exact OR - else - Inexact OR / Inexact AND / Exact AND
                if exact && filter.match_any {
                    let comparator = match blacklist {
                        true => "NOT IN",
                        false => "IN",
                    };
                    where_clauses.push(format!("game.{} {} rarray(?)", field_name, comparator));
                    params.push(SearchParam::StringVec(value_list.clone()));
                } else if blacklist {
                    let mut inner_clauses = vec![];
                    for value in value_list {
                        inner_clauses.push(format!("game.{} {} ?", field_name, comparator));
                        if exact {
                            params.push(SearchParam::String(value.clone()));
                        } else {
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }
                    }
                    where_clauses.push(format!("({})", inner_clauses.join(" AND ")));
                } else {
                    for value in value_list {
                        where_clauses.push(format!("game.{} {} ?", field_name, comparator));
                        if exact {
                            params.push(SearchParam::String(value.clone()));
                        } else {
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }
                    }
                }
            }
        };

    // exact whitelist
    exact_whitelist_clause!(add_clause, "library", &filter.exact_whitelist.library);
    exact_whitelist_clause!(add_clause, "developer", &filter.exact_whitelist.developer);
    exact_whitelist_clause!(add_clause, "publisher", &filter.exact_whitelist.publisher);
    exact_whitelist_clause!(add_clause, "series", &filter.exact_whitelist.series);
    exact_whitelist_clause!(add_clause, "playMode", &filter.exact_whitelist.play_mode);
    exact_whitelist_clause!(add_clause, "status", &filter.exact_whitelist.status);
    exact_whitelist_clause!(add_clause, "notes", &filter.exact_whitelist.notes);
    exact_whitelist_clause!(add_clause, "source", &filter.exact_whitelist.source);
    exact_whitelist_clause!(
        add_clause,
        "originalDescription",
        &filter.exact_whitelist.original_description
    );
    exact_whitelist_clause!(add_clause, "language", &filter.exact_whitelist.language);
    exact_whitelist_clause!(
        add_clause,
        "ruffleSupport",
        &filter.exact_whitelist.ruffle_support
    );

    // exact blacklist
    exact_blacklist_clause!(add_clause, "library", &filter.exact_blacklist.library);
    exact_blacklist_clause!(add_clause, "developer", &filter.exact_blacklist.developer);
    exact_blacklist_clause!(add_clause, "publisher", &filter.exact_blacklist.publisher);
    exact_blacklist_clause!(add_clause, "series", &filter.exact_blacklist.series);
    exact_blacklist_clause!(add_clause, "playMode", &filter.exact_blacklist.play_mode);
    exact_blacklist_clause!(add_clause, "status", &filter.exact_blacklist.status);
    exact_blacklist_clause!(add_clause, "notes", &filter.exact_blacklist.notes);
    exact_blacklist_clause!(add_clause, "source", &filter.exact_blacklist.source);
    exact_blacklist_clause!(
        add_clause,
        "originalDescription",
        &filter.exact_blacklist.original_description
    );
    exact_blacklist_clause!(add_clause, "language", &filter.exact_blacklist.language);
    exact_blacklist_clause!(
        add_clause,
        "ruffleSupport",
        &filter.exact_blacklist.ruffle_support
    );

    // whitelist
    whitelist_clause!(add_clause, "library", &filter.whitelist.library);
    whitelist_clause!(add_clause, "developer", &filter.whitelist.developer);
    whitelist_clause!(add_clause, "publisher", &filter.whitelist.publisher);
    whitelist_clause!(add_clause, "series", &filter.whitelist.series);
    whitelist_clause!(add_clause, "playMode", &filter.whitelist.play_mode);
    whitelist_clause!(add_clause, "status", &filter.whitelist.status);
    whitelist_clause!(add_clause, "notes", &filter.whitelist.notes);
    whitelist_clause!(add_clause, "source", &filter.whitelist.source);
    whitelist_clause!(
        add_clause,
        "originalDescription",
        &filter.whitelist.original_description
    );
    whitelist_clause!(add_clause, "language", &filter.whitelist.language);
    whitelist_clause!(
        add_clause,
        "ruffleSupport",
        &filter.whitelist.ruffle_support
    );

    // blacklist
    blacklist_clause!(add_clause, "library", &filter.blacklist.library);
    blacklist_clause!(add_clause, "developer", &filter.blacklist.developer);
    blacklist_clause!(add_clause, "publisher", &filter.blacklist.publisher);
    blacklist_clause!(add_clause, "series", &filter.blacklist.series);
    blacklist_clause!(add_clause, "playMode", &filter.blacklist.play_mode);
    blacklist_clause!(add_clause, "status", &filter.blacklist.status);
    blacklist_clause!(add_clause, "notes", &filter.blacklist.notes);
    blacklist_clause!(add_clause, "source", &filter.blacklist.source);
    blacklist_clause!(
        add_clause,
        "originalDescription",
        &filter.blacklist.original_description
    );
    blacklist_clause!(add_clause, "language", &filter.blacklist.language);
    blacklist_clause!(
        add_clause,
        "ruffleSupport",
        &filter.blacklist.ruffle_support
    );

    let mut id_clause = |values: &Option<Vec<String>>, exact: bool, blacklist: bool| {
        if let Some(value_list) = values {
            if exact {
                // All game ids are exact, AND would be impossible to satisfy, treat as OR, always
                let comparator = match blacklist {
                    true => "NOT IN",
                    false => "IN",
                };
                where_clauses.push(format!("(game.id {} rarray(?) OR game.id {} (SELECT id FROM game_redirect WHERE sourceId IN rarray(?)))", comparator, comparator));
                params.push(SearchParam::StringVec(value_list.clone()));
                params.push(SearchParam::StringVec(value_list.clone()));
            } else {
                for value in value_list {
                    if value.len() == 36 {
                        let comparator = match blacklist {
                            true => "!=",
                            false => "=",
                        };
                        where_clauses.push(format!("(game.id {} ? OR game.id {} (SELECT id FROM game_redirect WHERE sourceId = ? LIMIT 1))", comparator, comparator));

                        params.push(SearchParam::String(value.clone()));
                        params.push(SearchParam::String(value.clone()));
                    } else {
                        let comparator = match blacklist {
                            true => "NOT LIKE",
                            false => "LIKE",
                        };
                        where_clauses.push(format!("(game.id {} ?)", comparator));
                        let p = format!("%{}%", value);
                        params.push(SearchParam::String(p));
                    }
                }
            }
        }
    };

    id_clause(&filter.exact_whitelist.id, true, false);
    id_clause(&filter.exact_blacklist.id, true, true);
    id_clause(&filter.whitelist.id, false, false);
    id_clause(&filter.blacklist.id, false, false);

    let mut add_tagged_clause =
        |tag_name: &str, values: &Option<Vec<String>>, exact: bool, blacklist: bool| {
            if let Some(value_list) = values {
                let comparator = match blacklist {
                    true => "NOT IN",
                    false => "IN",
                };

                // Exact OR - else - Inexact OR / Inexact AND / Exact AND
                if exact && filter.match_any {
                    // Must be an exact OR
                    params.push(SearchParam::StringVec(value_list.clone()));

                    let tag_query = format!(
                        "game.id {} (SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                SELECT {}Id FROM {}_alias WHERE name IN rarray(?)))",
                        comparator, tag_name, tag_name, tag_name, tag_name, tag_name
                    );

                    where_clauses.push(tag_query);
                } else {
                    let mut inner_tag_queries = vec![];

                    // Add parameters
                    if exact {
                        for value in value_list {
                            inner_tag_queries.push("name = ?");
                            params.push(SearchParam::String(value.clone()));
                        }
                    } else {
                        for value in value_list {
                            inner_tag_queries.push("name LIKE ?");
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }
                    }

                    // Add query
                    let tag_query = match (blacklist, filter.match_any) {
                        (false, false) => {
                            if inner_tag_queries.len() == 1 {
                                format!(
                                    "game.id {} (SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                                SELECT {}Id FROM {}_alias WHERE {})
                            )",
                                    comparator,
                                    tag_name,
                                    tag_name,
                                    tag_name,
                                    tag_name,
                                    tag_name,
                                    inner_tag_queries[0]
                                )
                            } else {
                                let mut q = format!(
                                    "SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                                    SELECT {}Id FROM {}_alias WHERE {}
                                )",
                                    tag_name,
                                    tag_name,
                                    tag_name,
                                    tag_name,
                                    tag_name,
                                    inner_tag_queries[0]
                                );
                                for inner_tag_query in inner_tag_queries.iter().skip(1) {
                                    let part = format!(
                                        " AND gameId IN (
                                    SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                                        SELECT {}Id FROM {}_alias WHERE {}
                                    )
                                )",
                                        tag_name,
                                        tag_name,
                                        tag_name,
                                        tag_name,
                                        tag_name,
                                        inner_tag_query
                                    );
                                    q.push_str(&part);
                                }
                                format!("game.id {} ({})", comparator, q)
                            }
                        }
                        // Let blacklisted tags always use OR comparisons
                        // This needs to be changed to check for BOTH tags being on a game later!
                        (true, false) => format!(
                            "game.id {} (SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                    SELECT {}Id FROM {}_alias WHERE ({})))",
                            comparator,
                            tag_name,
                            tag_name,
                            tag_name,
                            tag_name,
                            tag_name,
                            inner_tag_queries.join(" OR ")
                        ),
                        (true, true) | (false, true) => format!(
                            "game.id {} (SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                    SELECT {}Id FROM {}_alias WHERE name IN {}))",
                            comparator,
                            tag_name,
                            tag_name,
                            tag_name,
                            tag_name,
                            tag_name,
                            inner_tag_queries.join(" OR ")
                        ),
                    };

                    where_clauses.push(tag_query);
                }
            }
        };

    // tag groups
    add_tagged_clause("tag", &filter.whitelist.tags, false, false);
    add_tagged_clause("tag", &filter.blacklist.tags, false, true);
    add_tagged_clause("tag", &filter.exact_whitelist.tags, true, false);
    add_tagged_clause("tag", &filter.exact_blacklist.tags, true, true);

    add_tagged_clause("platform", &filter.whitelist.platforms, false, false);
    add_tagged_clause("platform", &filter.blacklist.platforms, false, true);
    add_tagged_clause("platform", &filter.exact_whitelist.platforms, true, false);
    add_tagged_clause("platform", &filter.exact_blacklist.platforms, true, true);

    let mut add_multi_clause =
        |field_names: Vec<&str>, filter: &Option<Vec<String>>, exact: bool, blacklist: bool| {
            if let Some(value_list) = filter {
                let comparator = match (blacklist, exact) {
                    (true, true) => "!=",
                    (true, false) => "NOT LIKE",
                    (false, true) => "=",
                    (false, false) => "LIKE",
                };

                if blacklist {
                    let mut inner_clauses = vec![];
                    for value in value_list {
                        let mut value_clauses = vec![];
                        for field_name in field_names.clone() {
                            value_clauses.push(format!("game.{} {} ?", field_name, comparator));
                            if exact {
                                params.push(SearchParam::String(value.clone()));
                            } else {
                                let p = format!("%{}%", value);
                                params.push(SearchParam::String(p));
                            }
                        }
                        inner_clauses.push(format!("({})", &value_clauses.join(" OR ")));
                    }
                    where_clauses.push(format!("({})", inner_clauses.join(" OR ")));
                } else {
                    for value in value_list {
                        let mut value_clauses = vec![];
                        for field_name in field_names.clone() {
                            value_clauses.push(format!("game.{} {} ?", field_name, comparator));
                            if exact {
                                params.push(SearchParam::String(value.clone()));
                            } else {
                                let p = format!("%{}%", value);
                                params.push(SearchParam::String(p));
                            }
                        }
                        where_clauses.push(format!("({})", &value_clauses.join(" OR ")));
                    }
                }
            }
        };

    // whitelist
    add_multi_clause(
        vec!["title", "alternateTitles"],
        &filter.whitelist.title,
        false,
        false,
    );
    add_multi_clause(
        vec![
            "title",
            "alternateTitles",
            "developer",
            "publisher",
            "series",
        ],
        &filter.whitelist.generic,
        false,
        false,
    );

    // blacklist
    add_multi_clause(
        vec!["title", "alternateTitles"],
        &filter.blacklist.title,
        false,
        true,
    );
    add_multi_clause(
        vec![
            "title",
            "alternateTitles",
            "developer",
            "publisher",
            "series",
        ],
        &filter.blacklist.generic,
        false,
        true,
    );

    let mut add_joint_game_data_clause =
        |field_name: &str,
         game_field_name: &str,
         filter: &Option<Vec<String>>,
         exact: bool,
         blacklist: bool| {
            if let Some(value_list) = filter {
                let comparator = match (blacklist, exact) {
                    (true, true) => "!=",
                    (true, false) => "NOT LIKE",
                    (false, true) => "=",
                    (false, false) => "LIKE",
                };

                if blacklist {
                    let mut inner_clauses = vec![];
                    for value in value_list {
                        let mut value_clauses = vec![];
                        value_clauses.push(format!("game.{} {} ?", game_field_name, comparator));
                        if exact {
                            params.push(SearchParam::String(value.clone()));
                        } else {
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }

                        value_clauses.push(format!(
                            "game.id IN (SELECT gameId FROM game_data WHERE {} {} ?)",
                            field_name, comparator
                        ));
                        if exact {
                            params.push(SearchParam::String(value.clone()));
                        } else {
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }
                        inner_clauses.push(format!("({})", &value_clauses.join(" AND ")));
                    }
                    where_clauses.push(format!("({})", inner_clauses.join(" OR ")));
                } else {
                    for value in value_list {
                        let mut value_clauses = vec![];
                        value_clauses.push(format!("game.{} {} ?", game_field_name, comparator));
                        if exact {
                            params.push(SearchParam::String(value.clone()));
                        } else {
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }

                        value_clauses.push(format!(
                            "game.id IN (SELECT gameId FROM game_data WHERE {} {} ?)",
                            field_name, comparator
                        ));
                        if exact {
                            params.push(SearchParam::String(value.clone()));
                        } else {
                            let p = format!("%{}%", value);
                            params.push(SearchParam::String(p));
                        }
                        where_clauses.push(format!("({})", &value_clauses.join(" OR ")));
                    }
                }
            }
        };

    add_joint_game_data_clause(
        "applicationPath",
        "applicationPath",
        &filter.whitelist.application_path,
        false,
        false,
    );
    add_joint_game_data_clause(
        "applicationPath",
        "applicationPath",
        &filter.blacklist.application_path,
        false,
        true,
    );
    add_joint_game_data_clause(
        "applicationPath",
        "applicationPath",
        &filter.exact_whitelist.application_path,
        true,
        false,
    );
    add_joint_game_data_clause(
        "applicationPath",
        "applicationPath",
        &filter.exact_blacklist.application_path,
        true,
        true,
    );

    add_joint_game_data_clause(
        "launchCommand",
        "launchCommand",
        &filter.whitelist.launch_command,
        false,
        false,
    );
    add_joint_game_data_clause(
        "launchCommand",
        "launchCommand",
        &filter.blacklist.launch_command,
        false,
        true,
    );
    add_joint_game_data_clause(
        "launchCommand",
        "launchCommand",
        &filter.exact_whitelist.launch_command,
        true,
        false,
    );
    add_joint_game_data_clause(
        "launchCommand",
        "launchCommand",
        &filter.exact_blacklist.launch_command,
        true,
        true,
    );

    // Tag and Platform comparisons
    let mut add_compare_tag_clause = |field_name: &str,
                                      comparator: KeyChar,
                                      filter: &Option<i64>| {
        if let Some(f) = filter {
            if *f == 0 {
                match comparator {
                    KeyChar::EQUALS => {
                        // Select games with exactly 0 additional apps
                        where_clauses.push(format!(
                            "game.id NOT IN (SELECT gameId FROM game_{}s_{})",
                            field_name, field_name
                        ));
                    }
                    KeyChar::LOWER => (),
                    KeyChar::HIGHER => {
                        // Select games with 1 or more additional apps
                        where_clauses.push(format!(
                            "game.id IN (SELECT gameId FROM game_{}s_{})",
                            field_name, field_name
                        ));
                    }
                    KeyChar::MATCHES => (),
                }
            } else {
                match comparator {
                    KeyChar::MATCHES => (),
                    KeyChar::LOWER => {
                        where_clauses.push(format!("game.id NOT IN (SELECT gameId FROM game_{}s_{} GROUP BY gameId HAVING COUNT(gameId) >= ?)", field_name, field_name));
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::HIGHER => {
                        where_clauses.push(format!("game.id IN (SELECT gameId FROM game_{}s_{} GROUP BY gameId HAVING COUNT(gameId) > ?)", field_name, field_name));
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::EQUALS => {
                        where_clauses.push(format!("game.id IN (SELECT gameId FROM game_{}s_{} GROUP BY gameId HAVING COUNT(gameId) = ?)", field_name, field_name));
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                }
            }
        }
    };

    add_compare_tag_clause("tag", KeyChar::LOWER, &filter.lower_than.tags);
    add_compare_tag_clause("tag", KeyChar::HIGHER, &filter.higher_than.tags);
    add_compare_tag_clause("tag", KeyChar::EQUALS, &filter.equal_to.tags);

    add_compare_tag_clause("platform", KeyChar::LOWER, &filter.lower_than.platforms);
    add_compare_tag_clause("platform", KeyChar::HIGHER, &filter.higher_than.platforms);
    add_compare_tag_clause("platform", KeyChar::EQUALS, &filter.equal_to.platforms);

    // Add app comparisons
    let mut add_compare_add_app_clause = |comparator: KeyChar, filter: &Option<i64>| {
        if let Some(f) = filter {
            if *f == 0 {
                match comparator {
                    KeyChar::EQUALS => {
                        // Select games with exactly 0 additional apps
                        where_clauses.push(
                            "game.id NOT IN (SELECT parentGameId FROM additional_app)".to_string(),
                        );
                    }
                    KeyChar::LOWER => (),
                    KeyChar::HIGHER => {
                        // Select games with 1 or more additional apps
                        where_clauses.push(
                            "game.id IN (SELECT parentGameId FROM additional_app)".to_string(),
                        );
                    }
                    KeyChar::MATCHES => (),
                }
            } else {
                match comparator {
                    KeyChar::MATCHES => (),
                    KeyChar::LOWER => {
                        where_clauses.push("game.id NOT IN (SELECT parentGameId FROM additional_app GROUP BY parentGameId HAVING COUNT(parentGameId) >= ?)".to_string());
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::HIGHER => {
                        where_clauses.push("game.id IN (SELECT parentGameId FROM additional_app GROUP BY parentGameId HAVING COUNT(parentGameId) > ?)".to_string());
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::EQUALS => {
                        where_clauses.push("game.id IN (SELECT parentGameId FROM additional_app GROUP BY parentGameId HAVING COUNT(parentGameId) = ?)".to_string());
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                }
            }
        }
    };

    add_compare_add_app_clause(KeyChar::LOWER, &filter.lower_than.add_apps);
    add_compare_add_app_clause(KeyChar::HIGHER, &filter.higher_than.add_apps);
    add_compare_add_app_clause(KeyChar::EQUALS, &filter.equal_to.add_apps);

    let mut add_compare_game_data_clause = |comparator: KeyChar, filter: &Option<i64>| {
        if let Some(f) = filter {
            if *f <= 0 {
                match comparator {
                    KeyChar::EQUALS => {
                        // Select games with exactly 0 additional apps
                        where_clauses
                            .push("game.id NOT IN (SELECT gameId FROM game_data)".to_string());
                    }
                    KeyChar::LOWER => (),
                    KeyChar::HIGHER => {
                        // Select games with 1 or more additional apps
                        where_clauses.push("game.id IN (SELECT gameId FROM game_data)".to_string());
                    }
                    KeyChar::MATCHES => (),
                }
            } else {
                match comparator {
                    KeyChar::MATCHES => (),
                    KeyChar::LOWER => {
                        where_clauses.push("game.id NOT IN (SELECT gameId FROM game_data GROUP BY gameId HAVING COUNT(gameId) >= ?)".to_string());
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::HIGHER => {
                        where_clauses.push("game.id IN (SELECT gameId FROM game_data GROUP BY gameId HAVING COUNT(gameId) > ?)".to_string());
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::EQUALS => {
                        where_clauses.push("game.id IN (SELECT gameId FROM game_data GROUP BY gameId HAVING COUNT(gameId) = ?)".to_string());
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                }
            }
        }
    };

    add_compare_game_data_clause(KeyChar::LOWER, &filter.lower_than.game_data);
    add_compare_game_data_clause(KeyChar::HIGHER, &filter.higher_than.game_data);
    add_compare_game_data_clause(KeyChar::EQUALS, &filter.equal_to.game_data);

    let mut add_compare_dates_clause =
        |date_field: &str, comparator: KeyChar, filter: &Option<String>| {
            if let Some(f) = filter {
                match comparator {
                    KeyChar::MATCHES => (),
                    KeyChar::LOWER => {
                        where_clauses.push(format!("date(game.{}) < ?", date_field));
                        params.push(SearchParam::String(f.clone()));
                    }
                    KeyChar::HIGHER => {
                        // e.g "2021-01" will generate >= "2021-01" and < "2021-02"
                        where_clauses.push(format!("date(game.{}) >= ?", date_field));
                        params.push(SearchParam::String(f.clone()));
                    }
                    KeyChar::EQUALS => {
                        where_clauses.push(format!("date(game.{}) LIKE ?", date_field));
                        let p = f.clone() + "%";
                        params.push(SearchParam::String(p));
                    }
                }
            }
        };

    add_compare_dates_clause("dateAdded", KeyChar::LOWER, &filter.lower_than.date_added);
    add_compare_dates_clause("dateAdded", KeyChar::HIGHER, &filter.higher_than.date_added);
    add_compare_dates_clause("dateAdded", KeyChar::EQUALS, &filter.equal_to.date_added);

    add_compare_dates_clause(
        "dateModified",
        KeyChar::LOWER,
        &filter.lower_than.date_modified,
    );
    add_compare_dates_clause(
        "dateModified",
        KeyChar::HIGHER,
        &filter.higher_than.date_modified,
    );
    add_compare_dates_clause(
        "dateModified",
        KeyChar::EQUALS,
        &filter.equal_to.date_modified,
    );

    add_compare_dates_clause("lastPlayed", KeyChar::LOWER, &filter.lower_than.last_played);
    add_compare_dates_clause(
        "lastPlayed",
        KeyChar::HIGHER,
        &filter.higher_than.last_played,
    );
    add_compare_dates_clause("lastPlayed", KeyChar::EQUALS, &filter.equal_to.last_played);

    let mut add_compare_dates_string_clause =
        |date_field: &str, comparator: KeyChar, filter: &Option<String>| {
            if let Some(f) = filter {
                match comparator {
                    KeyChar::MATCHES => (),
                    KeyChar::LOWER => {
                        where_clauses.push(format!("game.{} < ?", date_field));
                        params.push(SearchParam::String(f.clone()));
                    }
                    KeyChar::HIGHER => {
                        // e.g "2021-01" will generate >= "2021-01" and < "2021-02"
                        where_clauses.push(format!("game.{} >= ?", date_field));
                        params.push(SearchParam::String(f.clone()));
                    }
                    KeyChar::EQUALS => {
                        where_clauses.push(format!("game.{} LIKE ?", date_field));
                        let p = f.clone() + "%";
                        params.push(SearchParam::String(p));
                    }
                }
            }
        };

    add_compare_dates_string_clause(
        "releaseDate",
        KeyChar::LOWER,
        &filter.lower_than.release_date,
    );
    add_compare_dates_string_clause(
        "releaseDate",
        KeyChar::HIGHER,
        &filter.higher_than.release_date,
    );
    add_compare_dates_string_clause(
        "releaseDate",
        KeyChar::EQUALS,
        &filter.equal_to.release_date,
    );

    let mut add_compare_counter_clause =
        |counter: &str, comparator: KeyChar, filter: &Option<i64>| {
            if let Some(f) = filter {
                match comparator {
                    KeyChar::MATCHES => (),
                    KeyChar::LOWER => {
                        where_clauses.push(format!("game.{} < ?", counter));
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::HIGHER => {
                        where_clauses.push(format!("game.{} > ?", counter));
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                    KeyChar::EQUALS => {
                        where_clauses.push(format!("game.{} = ?", counter));
                        params.push(SearchParam::Integer64(f.clone()));
                    }
                }
            }
        };

    add_compare_counter_clause("playtime", KeyChar::LOWER, &filter.lower_than.playtime);
    add_compare_counter_clause("playtime", KeyChar::HIGHER, &filter.higher_than.playtime);
    add_compare_counter_clause("playtime", KeyChar::EQUALS, &filter.equal_to.playtime);

    add_compare_counter_clause("playCounter", KeyChar::LOWER, &filter.lower_than.playcount);
    add_compare_counter_clause(
        "playCounter",
        KeyChar::HIGHER,
        &filter.higher_than.playcount,
    );
    add_compare_counter_clause("playCounter", KeyChar::EQUALS, &filter.equal_to.playcount);

    // Installed clause
    if let Some(val) = filter.bool_comp.installed {
        where_clauses.push(
            "game.id IN (SELECT gameId FROM game_data WHERE game_data.presentOnDisk = ?)"
                .to_owned(),
        );
        params.push(SearchParam::Boolean(val));
    }

    // Deal with complicated extension comparisons

    let mut ext_add_clause = |values: &Option<HashMap<String, HashMap<String, Vec<String>>>>,
                              exact: bool,
                              blacklist: bool| {
        if let Some(value_list) = values {
            let comparator = match (blacklist, exact) {
                (true, true) => "!=",
                (true, false) => "NOT LIKE",
                (false, true) => "=",
                (false, false) => "LIKE",
            };

            // Exact OR - else - Inexact OR / Inexact AND / Exact AND
            if exact && filter.match_any {
                let comparator = match blacklist {
                    true => "NOT IN",
                    false => "IN",
                };
                for (ext_id, comp) in value_list {
                    for (key, value_list) in comp {
                        where_clauses.push(
                            format!("game.id IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') {} rarray(?))", key, comparator)
                        );
                        params.push(SearchParam::String(ext_id.clone()));
                        params.push(SearchParam::StringVec(value_list.clone()));
                    }
                }
            } else if blacklist {
                let mut inner_clauses = vec![];
                for (ext_id, comp) in value_list {
                    for (key, value_list) in comp {
                        for value in value_list {
                            inner_clauses.push(
                                format!("game.id IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') {} ?)", key, comparator)
                            );
                            params.push(SearchParam::String(ext_id.clone()));
                            if exact {
                                params.push(SearchParam::String(value.clone()));
                            } else {
                                let p = format!("%{}%", value);
                                params.push(SearchParam::String(p));
                            }
                        }
                    }
                }
                where_clauses.push(format!("({})", inner_clauses.join(" AND ")));
            } else {
                for (ext_id, comp) in value_list {
                    for (key, value_list) in comp {
                        for value in value_list {
                            where_clauses.push(
                                format!("game.id IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') {} ?)", key, comparator)
                            );
                            params.push(SearchParam::String(ext_id.clone()));
                            if exact {
                                params.push(SearchParam::String(value.clone()));
                            } else {
                                let p = format!("%{}%", value);
                                params.push(SearchParam::String(p));
                            }
                        }
                    }
                }
            }
        }
    };

    // Ext strings

    ext_add_clause(&filter.whitelist.ext, false, false);
    ext_add_clause(&filter.blacklist.ext, false, true);
    ext_add_clause(&filter.exact_whitelist.ext, true, false);
    ext_add_clause(&filter.exact_blacklist.ext, true, true);

    let mut ext_add_compare =
    |comparator: KeyChar, value: &Option<HashMap<String, HashMap<String, i64>>>| {
        if let Some(value_list) = value {
            for (ext_id, values) in value_list {
                for (key, f) in values {
                    match comparator {
                        KeyChar::EQUALS | KeyChar::MATCHES => {
                            where_clauses.push(format!("game.id IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') = ?)", key).to_owned());
                            params.push(SearchParam::String(ext_id.clone()));
                            params.push(SearchParam::Integer64(f.clone()));
                        },
                        KeyChar::LOWER => {
                            where_clauses.push(format!("game.id NOT IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') >= ?)", key).to_owned());
                            params.push(SearchParam::String(ext_id.clone()));
                            params.push(SearchParam::Integer64(f.clone()));
                        }
                        KeyChar::HIGHER => {
                            where_clauses.push(format!("game.id IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') > ?)", key).to_owned());
                            params.push(SearchParam::String(ext_id.clone()));
                            params.push(SearchParam::Integer64(f.clone()));
                        }
                    }
                }
            }
        }
    };

    // Ext numericals

    ext_add_compare(KeyChar::EQUALS, &filter.equal_to.ext);
    ext_add_compare(KeyChar::LOWER, &filter.lower_than.ext);
    ext_add_compare(KeyChar::HIGHER, &filter.higher_than.ext);

    // Ext bools

    if let Some(value_list) = &filter.bool_comp.ext {
        for (ext_id, comp) in value_list {
            for (key, value) in comp {
                where_clauses.push(
                    format!("game.id IN (SELECT gameId FROM ext_data WHERE extId = ? AND JSON_EXTRACT(data, '$.{}') = ?)", key).to_owned()
                );
                params.push(SearchParam::String(ext_id.clone()));
                params.push(SearchParam::Boolean(value.clone()));
            }
        }
    }

    // Remove any cases of "()" from where_clauses

    where_clauses = where_clauses.into_iter().filter(|s| s != "()").collect();

    if filter.match_any {
        where_clauses.join(" OR ")
    } else {
        where_clauses.join(" AND ")
    }
}

fn format_query(query: &str, substitutions: Vec<SearchParam>) -> String {
    let mut formatted_query = String::new();
    let mut trim_mode = false;
    let mut indent = 0;
    let mut substitution_iter = substitutions.iter();
    let mut skip_drop = false;

    for (idx, ch) in query.chars().enumerate() {
        match ch {
            '(' => {
                if idx + 1 < query.len() {
                    let next: String = query.chars().skip(idx + 1).take(1).collect();
                    if vec![")", "*"].contains(&next.as_str()) {
                        formatted_query.push(ch);
                        skip_drop = true;
                        continue;
                    }
                }
                indent += 4;
                trim_mode = true;
                formatted_query.push(ch);
                formatted_query.push('\n');
            }
            ')' => {
                if skip_drop {
                    skip_drop = false;
                    formatted_query.push(ch);
                    continue;
                }
                trim_mode = false;
                indent -= 4;
                formatted_query.push('\n');
                let spaces = " ".repeat(indent);
                formatted_query.push_str(&spaces);
                formatted_query.push(ch);
            }
            '?' => {
                if let Some(subst) = substitution_iter.next() {
                    let wrapped_subst = format!("'{}'", subst);
                    formatted_query.push_str(&wrapped_subst);
                } else {
                    // If there are no more substitutions, keep the '?' or handle as needed
                    formatted_query.push(ch);
                }
            }
            ' ' => {
                if !trim_mode {
                    formatted_query.push(ch);
                }
            }
            '\n' => trim_mode = true,
            _ => {
                if trim_mode {
                    let spaces = " ".repeat(indent);
                    formatted_query.push_str(&spaces);
                    trim_mode = false;
                }
                formatted_query.push(ch);
            }
        }
    }

    formatted_query
}

pub fn new_custom_id_order(conn: &Connection, custom_id_order: Vec<String>) -> Result<()> {
    let new_order = custom_id_order.join(";");
    let current_order = conn.query_row("SELECT IFNULL(string_agg(id, ';'), ''),  ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS RowNum FROM custom_id_order ORDER BY RowNum", (), |row| row.get::<_, String>(0))?;
    if current_order != new_order {
        conn.execute("DELETE FROM custom_id_order", ())?;
        let mut stmt = conn.prepare("INSERT INTO custom_id_order (id) VALUES (?)")?;
        for id in custom_id_order {
            stmt.execute(params![id])?;
        }
    }
    Ok(())
}

// Dumb replacment string to denote an 'empty' value
const REPLACEMENT: &str =
    "UIOWHDYUAWDGBAWYUODIGAWYUIDIAWGHDYUI8AWGHDUIAWDHNAWUIODHJNAWIOUDHJNAWOUIDAJNWMLDK";

pub fn new_tag_filter_index(conn: &Connection, search: &mut GameSearch) -> Result<()> {
    // Allow use of rarray() in SQL queries
    rusqlite::vtab::array::load_module(conn)?;

    search.limit = 9999999999999999;
    search.filter = GameFilter::default();
    search.filter.match_any = true;

    if let Some(t) = search.with_tag_filter.clone() {
        if t.len() > 0 {
            search.filter.exact_blacklist.tags = Some(t);
            search.with_tag_filter = None;
        }
    }

    if search.filter.exact_blacklist.tags.is_none()
        || search.filter.exact_blacklist.tags.clone().unwrap().len() == 0
    {
        return Ok(());
    }

    let mut tags = search.filter.exact_blacklist.tags.clone().unwrap();
    tags.sort();
    let tags_key = tags.join(";");

    // Check against existing key
    let tag_filter_info = conn
        .query_row("SELECT key, dirty FROM tag_filter_index_info", (), |row| {
            Ok(TagFilterInfo {
                key: row.get(0)?,
                dirty: row.get(1)?,
            })
        })
        .optional()?;

    match tag_filter_info {
        Some(info) => {
            // Index already built and clean, return
            if !info.dirty && tags_key == info.key {
                return Ok(());
            }
        }
        None => {
            // No existing index, continue
        }
    }

    debug_println!("filtering {} tags", tags.len());

    conn.execute("DELETE FROM tag_filter_index", ())?; // Empty existing index

    let (query, params) = build_search_query(search, TAG_FILTER_INDEX_QUERY);

    // Convert the parameters array to something rusqlite understands
    let params_as_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    debug_println!(
        "new filtered tag query - \n{}",
        format_query(&query, params.clone())
    );

    let mut stmt = conn.prepare(query.as_str())?;
    stmt.execute(params_as_refs.as_slice())?;

    tags.sort();

    conn.execute("DELETE FROM tag_filter_index_info", ())?; // Empty existing index info
    conn.execute(
        "INSERT INTO tag_filter_index_info (key, dirty) VALUES (?, 0)",
        params![tags_key],
    )?;

    Ok(())
}

pub fn mark_index_dirty(conn: &Connection) -> Result<()> {
    conn.execute("UPDATE tag_filter_index_info SET dirty = 1", ())?;
    Ok(())
}

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(not(feature = "napi"), derive(Clone))]
#[derive(Debug)]
pub enum ElementType {
    MODIFIER,
    KEY,
    KEYCHAR,
    VALUE,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct ElementPosition {
    pub element: ElementType,
    pub value: String,
    pub start: i32,
    pub end: i32,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct ParsedInput {
    pub search: GameSearch,
    pub positions: Vec<ElementPosition>,
}

pub fn parse_user_input(input: &str, ext_searchables: Option<&HashMap<String, ExtSearchableRegistered>>) -> ParsedInput {
    let ext_searchables = match ext_searchables {
        Some(e) => e,
        None => &HashMap::new()
    };

    let mut search = GameSearch::default();
    let mut filter = ForcedGameFilter::default();

    let mut capturing_quotes = false;
    let mut working_key = String::new();
    let mut working_value = String::new();
    let mut working_key_char: Option<KeyChar> = None;
    let mut negative = false;

    let mut positions = Vec::new();
    let mut current_pos = 0;

    for raw_token in input.split(" ") {
        // Value on the same scope as token to append to
        let mut token = raw_token.to_owned();
        let mut token_start = current_pos.try_into().unwrap_or(0);
        let mut _t = "".to_owned();
        debug_println!("token {}", token);
        // Handle continued value capture if needed

        if !capturing_quotes && token.len() > 1 {
            // Not inside quotes, check for negation
            if token.starts_with("-") {
                negative = true;

                token = token.strip_prefix("-").unwrap().to_owned();
                positions.push(ElementPosition {
                    element: ElementType::MODIFIER,
                    value: "-".to_owned(),
                    start: token_start,
                    end: token_start + 1,
                });
                token_start += 1;
            }

            if token.len() > 1 {
                debug_println!("checking token start");
                // Check for quick search options preceding token
                let ch = token.chars().next().unwrap();
                debug_println!("start char: {}", ch);
                match ch {
                    '#' => {
                        token = token.strip_prefix('#').unwrap().to_owned();
                        working_key = "tag".to_owned();
                        positions.push(ElementPosition {
                            element: ElementType::MODIFIER,
                            value: "#".to_owned(),
                            start: token_start,
                            end: token_start + 1,
                        });
                        token_start += 1;
                    }
                    '!' => {
                        token = token.strip_prefix('!').unwrap().to_owned();
                        working_key = "platform".to_owned();
                        positions.push(ElementPosition {
                            element: ElementType::MODIFIER,
                            value: "!".to_owned(),
                            start: token_start,
                            end: token_start + 1,
                        });
                        token_start += 1;
                    }
                    '@' => {
                        token = token.strip_prefix('@').unwrap().to_owned();
                        working_key = "developer".to_owned();
                        positions.push(ElementPosition {
                            element: ElementType::MODIFIER,
                            value: "@".to_owned(),
                            start: token_start,
                            end: token_start + 1,
                        });
                        token_start += 1;
                    }
                    _ => (),
                }
            }
        }

        if token.starts_with('"') {
            token = token.strip_prefix('"').unwrap().to_owned();
            // Opening quote
            capturing_quotes = true;
        }

        if capturing_quotes {
            // Inside quotes, add to working value
            if working_value == "" {
                // Start of value
                working_value = token.to_owned();
            } else {
                // Continued value
                working_value.push_str(&format!(" {}", token));
            }
        }

        if token.ends_with('"') && capturing_quotes {
            // Closing quote
            capturing_quotes = false;
            // Remove quote at end of working value, if doesn't exist then it's a broken quoted value
            working_value = working_value.strip_suffix('"').unwrap().to_owned();
        }

        if capturing_quotes {
            // Still in capture mode, get next token
            current_pos += raw_token.len() + 1;
            continue;
        }

        if working_value == "" {
            // No working input yet, check for key
            working_key_char = earliest_key_char(&token);

            // Extract the working key
            if let Some(kc) = working_key_char.clone() {
                let s: String = kc.into();
                let token_parts = token.split(&s).collect::<Vec<&str>>();
                if token_parts.len() > 1 {
                    // Has a key
                    debug_println!("key {:?}", &token_parts[0]);
                    working_key = token_parts[0].to_owned();
                    token = token_parts
                        .into_iter()
                        .skip(1)
                        .collect::<Vec<&str>>()
                        .join(&s);
                    debug_println!("value {:?}", &token);
                    positions.push(ElementPosition {
                        element: ElementType::KEY,
                        value: working_key.clone(),
                        start: token_start,
                        end: token_start + working_key.len().try_into().unwrap_or(0),
                    });
                    token_start += working_key.len().try_into().unwrap_or(0);
                } else {
                    token = token_parts[0].to_owned();
                }
            }

            // Single value, must be value
            if token.starts_with('"') && token.ends_with('"') {
                // Special case for empty value
                if token.len() == 2 {
                    if working_key != "" {
                        // Has a key, must be a deliberately empty value
                        working_value = REPLACEMENT.to_owned();
                    }
                } else {
                    // Fully inside quotes
                    token = token.strip_prefix('"').unwrap_or_else(|| "").to_owned();
                    token = token.strip_suffix('"').unwrap_or_else(|| "").to_owned();
                    working_value = token.to_owned();
                }
            } else {
                if token.starts_with('"') {
                    // Starts quotes
                    token = token.strip_prefix('"').unwrap().to_owned();
                    capturing_quotes = true;
                    working_value = token.to_owned();
                    continue;
                }
                working_value = token.to_owned();
            }
        }

        if working_value != "" {
            let mut exact = false;
            if working_key != "" {
                if working_value == REPLACEMENT {
                    // Is an empty replacement value, swap it back in now we know it exists
                    working_value = "".to_owned();
                    exact = true;
                } else {
                    if let Some(kc) = &working_key_char {
                        match kc {
                            KeyChar::EQUALS => exact = true,
                            _ => (),
                        }
                    }
                }
            }

            debug_println!(
                "key: {}, value: {}, negative: {}, exact: {}",
                working_key,
                working_value,
                negative,
                exact,
            );

            let mut list = match (negative, exact) {
                (true, false) => filter.blacklist.clone(),
                (false, false) => filter.whitelist.clone(),
                (true, true) => filter.exact_blacklist.clone(),
                (false, true) => filter.exact_whitelist.clone(),
            };
            let value = working_value.clone();

            if let Some(kc) = &working_key_char {
                positions.push(ElementPosition {
                    element: ElementType::KEYCHAR,
                    value: kc.to_owned().into(),
                    start: token_start,
                    end: token_start + 1,
                });
                token_start += 1;
            }

            // Track position of the value
            positions.push(ElementPosition {
                element: ElementType::VALUE,
                value: working_value.clone(),
                start: token_start,
                end: token_start + working_value.len().try_into().unwrap_or(0),
            });

            // Handle boolean comparisons
            let mut processed: bool = true;
            
            match working_key.to_lowercase().as_str() {
                "installed" => {
                    let mut value = !(working_value.to_lowercase() == "no"
                        && working_value.to_lowercase() == "false"
                        && working_value.to_lowercase() == "0");
                    if negative {
                        value = !value;
                    }

                    filter.bool_comp.installed = Some(value);
                }
                _ => {
                    // Check if this is a searchable key registered by an extension
                    if let Some(ext_searchable) = ext_searchables.get(working_key.to_lowercase().as_str()) {
                        if ext_searchable.value_type == ExtSearchableType::Boolean {
                            let mut value = !(working_value.to_lowercase() == "no"
                                && working_value.to_lowercase() == "false"
                                && working_value.to_lowercase() == "0");
                            if negative {
                                value = !value;
                            }

                            // Unwrap or create a new extensions filter
                            let mut inner_filter = filter.bool_comp.ext.unwrap_or_default();
                            // Insert a new map for the extension that owns this searchable if missing
                            let ext_filter = inner_filter.insert_or_get(ext_searchable.ext_id.clone());
                            ext_filter.insert(ext_searchable.key.clone(), value);
                            filter.bool_comp.ext = Some(inner_filter);
                        } else {
                            processed = false;
                        }
                    } else {
                        processed = false;
                    }            
                }
            }

            // Handle numerical comparisons
            if !processed {
                if let Some(kc) = &working_key_char {
                    processed = true;
                    match kc {
                        KeyChar::LOWER => {
                            let value = coerce_to_i64(&working_value);
                            match working_key.to_lowercase().as_str() {
                                "tags" => filter.lower_than.tags = Some(value),
                                "platforms" => filter.lower_than.platforms = Some(value),
                                "dateadded" | "da" => {
                                    filter.lower_than.date_added = Some(working_value.clone())
                                }
                                "datemodified" | "dm" => {
                                    filter.lower_than.date_modified = Some(working_value.clone())
                                }
                                "releasedate" | "rd" => {
                                    filter.lower_than.release_date = Some(working_value.clone())
                                }
                                "gamedata" | "gd" => filter.lower_than.game_data = Some(value),
                                "addapps" | "aa" => filter.lower_than.add_apps = Some(value),
                                "playtime" | "pt" => filter.lower_than.playtime = Some(value),
                                "playcount" | "pc" => filter.lower_than.playcount = Some(value),
                                "lastplayed" | "lp" => {
                                    filter.lower_than.last_played = Some(working_value.clone())
                                }
                                _ => {
                                    // Check if this is a searchable key registered by an extension
                                    if let Some(ext_searchable) = ext_searchables.get(working_key.to_lowercase().as_str()) {
                                        if ext_searchable.value_type == ExtSearchableType::Number {
                                            // Unwrap or create a new extensions filter
                                            let mut inner_filter = filter.lower_than.ext.unwrap_or_default();
                                            // Insert a new map for the extension that owns this searchable if missing
                                            let ext_filter = inner_filter.insert_or_get(ext_searchable.ext_id.clone());
                                            ext_filter.insert(ext_searchable.key.clone(), value);
                                            filter.lower_than.ext = Some(inner_filter);
                                        } else {
                                            processed = false;
                                        }
                                    } else {
                                        processed = false;
                                    }                           
                                }
                            }
                        }
                        KeyChar::HIGHER => {
                            let value = coerce_to_i64(&working_value);
                            match working_key.to_lowercase().as_str() {
                                "tags" => filter.higher_than.tags = Some(value),
                                "platforms" => filter.higher_than.platforms = Some(value),
                                "dateadded" | "da" => {
                                    filter.higher_than.date_added = Some(working_value.clone())
                                }
                                "datemodified" | "dm" => {
                                    filter.higher_than.date_modified = Some(working_value.clone())
                                }
                                "releasedate" | "rd" => {
                                    filter.higher_than.release_date = Some(working_value.clone())
                                }
                                "gamedata" | "gd" => filter.higher_than.game_data = Some(value),
                                "addapps" | "aa" => filter.higher_than.add_apps = Some(value),
                                "playtime" | "pt" => filter.higher_than.playtime = Some(value),
                                "playcount" | "pc" => filter.higher_than.playcount = Some(value),
                                "lastplayed" | "lp" => {
                                    filter.higher_than.last_played = Some(working_value.clone())
                                }
                                _ => {
                                    // Check if this is a searchable key registered by an extension
                                    if let Some(ext_searchable) = ext_searchables.get(working_key.to_lowercase().as_str()) {
                                        if ext_searchable.value_type == ExtSearchableType::Number {
                                            // Unwrap or create a new extensions filter
                                            let mut inner_filter = filter.higher_than.ext.unwrap_or_default();
                                            // Insert a new map for the extension that owns this searchable if missing
                                            let ext_filter = inner_filter.insert_or_get(ext_searchable.ext_id.clone());
                                            ext_filter.insert(ext_searchable.key.clone(), value);
                                            filter.higher_than.ext = Some(inner_filter);
                                        } else {
                                            processed = false;
                                        }
                                    } else {
                                        processed = false;
                                    }
                                }
                            }
                        }
                        KeyChar::MATCHES | KeyChar::EQUALS => {
                            let value = coerce_to_i64(&working_value);
                            match working_key.to_lowercase().as_str() {
                                "tags" => filter.equal_to.tags = Some(value),
                                "platforms" => filter.equal_to.platforms = Some(value),
                                "dateadded" | "da" => {
                                    filter.equal_to.date_added = Some(working_value.clone())
                                }
                                "datemodified" | "dm" => {
                                    filter.equal_to.date_modified = Some(working_value.clone())
                                }
                                "releasedate" | "rd" => {
                                    filter.equal_to.release_date = Some(working_value.clone())
                                }
                                "gamedata" | "gd" => filter.equal_to.game_data = Some(value),
                                "addapps" | "aa" => filter.equal_to.add_apps = Some(value),
                                "playtime" | "pt" => filter.equal_to.playtime = Some(value),
                                "playcount" | "pc" => filter.equal_to.playcount = Some(value),
                                "lastplayed" | "lp" => {
                                    filter.equal_to.last_played = Some(working_value.clone())
                                }
                                _ => {
                                    // Check if this is a searchable key registered by an extension
                                    if let Some(ext_searchable) = ext_searchables.get(working_key.to_lowercase().as_str()) {
                                        if ext_searchable.value_type == ExtSearchableType::Number {
                                            // Unwrap or create a new extensions filter
                                            let mut inner_filter = filter.equal_to.ext.unwrap_or_default();
                                            // Insert a new map for the extension that owns this searchable if missing
                                            let ext_filter = inner_filter.insert_or_get(ext_searchable.ext_id.clone());
                                            ext_filter.insert(ext_searchable.key.clone(), value);
                                            filter.equal_to.ext = Some(inner_filter);
                                        } else {
                                            processed = false;
                                        }
                                    } else {
                                        processed = false;
                                    }   
                                }
                            }
                        }
                    }
                }
            }

            // Handle generics and string matchers
            if !processed {
                // Has a complete value, add to filter
                match working_key.to_lowercase().as_str() {
                    "id" => list.id.push(value),
                    "lib" | "library" => list.library.push(value),
                    "title" => list.title.push(value),
                    "dev" | "developer" => list.developer.push(value),
                    "pub" | "publisher" => list.publisher.push(value),
                    "series" => list.series.push(value),
                    "tag" => list.tags.push(value),
                    "plat" | "platform" => list.platforms.push(value),
                    "mode" | "playmode" => list.play_mode.push(value),
                    "status" => list.status.push(value),
                    "note" | "notes" => list.notes.push(value),
                    "src" | "source" => list.source.push(value),
                    "od" | "desc" | "description" | "originaldescription" => {
                        list.original_description.push(value)
                    }
                    "lang" | "language" => list.language.push(value),
                    "ap" | "path" | "app" | "applicationpath" => list.application_path.push(value),
                    "lc" | "launchcommand" => list.launch_command.push(value),
                    "ruffle" | "rufflesupport" => list.ruffle_support.push(value.to_lowercase()),
                    _ => {
                        let processed = if let Some(ext_searchable) = ext_searchables.get(working_key.to_lowercase().as_str()) {
                            if ext_searchable.value_type == ExtSearchableType::String {
                                // Insert a new map for the extension that owns this searchable if missing
                                let ext_filter = list.ext.insert_or_get(ext_searchable.ext_id.clone());
                                let ext_list = ext_filter.insert_or_get(ext_searchable.key.clone());
                                ext_list.push(value.clone());

                                true
                            } else {
                                false
                            }
                        } else { 
                            false
                        };
                        if !processed {
                            // Reform the full search term if it contained a key character
                            let value = match &working_key_char {
                                Some(kc) => {
                                    let ks: String = kc.clone().into();
                                    let full_value = working_key.clone() + &ks + &value;
                                    full_value
                                }
                                None => value,
                            };

                            list.generic.push(value);
                        }
                    },
                }

                match (negative, exact) {
                    (true, false) => filter.blacklist = list,
                    (false, false) => filter.whitelist = list,
                    (true, true) => filter.exact_blacklist = list,
                    (false, true) => filter.exact_whitelist = list,
                }
            }

            negative = false;
            working_value.clear();
            working_key.clear();
        }
        current_pos += raw_token.len() + 1;
    }

    search.filter = (&filter).into();

    ParsedInput { search, positions }
}

#[derive(Debug, Clone, PartialEq)]
enum KeyChar {
    MATCHES,
    LOWER,
    HIGHER,
    EQUALS,
}

impl Into<String> for KeyChar {
    fn into(self) -> String {
        match self {
            KeyChar::MATCHES => ":".to_owned(),
            KeyChar::LOWER => "<".to_owned(),
            KeyChar::HIGHER => ">".to_owned(),
            KeyChar::EQUALS => "=".to_owned(),
        }
    }
}

const KEY_CHARS: [&str; 4] = [":", "<", ">", "="];

fn earliest_key_char(s: &str) -> Option<KeyChar> {
    let mut earliest_pos = None;
    let mut earliest_key_char = None;

    for key_char in KEY_CHARS {
        if let Some(pos) = s.find(key_char) {
            if earliest_pos.is_none() || pos < earliest_pos.unwrap() {
                earliest_pos = Some(pos);
                earliest_key_char = Some(key_char);
            }
        }
    }

    match earliest_key_char {
        Some(ekc) => match ekc {
            ":" => Some(KeyChar::MATCHES),
            "<" => Some(KeyChar::LOWER),
            ">" => Some(KeyChar::HIGHER),
            "=" => Some(KeyChar::EQUALS),
            _ => None,
        },
        None => None,
    }
}

fn coerce_to_i64(input: &str) -> i64 {
    // Substitute known replacements
    /* d - Seconds in a day
     * h - Seconds in an hour
     * m - seconds in a minute
     */

    // Insert '+' between consecutive time values (e.g., "1h30m" becomes "1h+30m")
    let insert_plus_re = Regex::new(r"(\d+)([yMwdhm])(?=\d)").unwrap();
    let mut processed_input = insert_plus_re
        .replace_all(&input, |caps: &Captures| {
            format!("{}{}+", &caps[1], &caps[2])
        })
        .to_string();

    let time_units = vec![
        ("y", 31_536_000), // years
        ("M", 2_592_000),  // months
        ("w", 604_800),    // weeks
        ("d", 86_400),     // days
        ("h", 3_600),      // hours
        ("m", 60),         // minutes
        ("s", 1),          // seconds
    ];

    // Replace each unit eg 30m with their seconds integer value e.g 1800
    for (unit, seconds) in time_units {
        let pattern = format!(r"(\d+){}", unit);
        let re = Regex::new(&pattern).unwrap();
        processed_input = re
            .replace_all(&processed_input, |caps: &Captures| {
                let time_value: i64 = caps[1].parse().unwrap_or(0); // Convert the captured group to i64
                (time_value * seconds).to_string() // Replace with time_value * seconds per unit
            })
            .to_string();
    }

    // Evaluate the mathematical expression we've made
    match meval::eval_str(&processed_input) {
        Ok(num) => num as i64,
        Err(_) => 0,
    }
}
