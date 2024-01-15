use rusqlite::{Connection, Result, OptionalExtension};

use crate::debug_println;

use super::{Game, get_game_platforms, get_game_tags, get_game_data};

#[derive(Debug, Clone)]
pub struct GameSearch {
    pub filter: GameFilter,
    pub load_relations: GameSearchRelations,
    pub order: GameSearchOrder,
    pub offset: Option<GameSearchOffset>,
    pub limit: i64,
    pub slim: bool,
}

#[derive(Debug, Clone)]
pub struct GameSearchOffset {
    pub value: String,
    pub game_id: String,
}

#[derive(Debug, Clone)]
pub struct GameSearchOrder {
    pub column: GameSearchSortable,
    pub direction: GameSearchDirection,
}

#[derive(Debug, Clone)]
pub enum GameSearchSortable {
    TITLE,
}

#[derive(Debug, Clone)]
pub enum GameSearchDirection {
    ASC,
    DESC,
}

#[derive(Debug, Clone)]
pub struct GameSearchRelations {
    pub tags: bool,
    pub platforms: bool,
    pub game_data: bool,
}

#[derive(Debug, Clone)]
pub struct GameFilter {
    pub subfilters: Vec<GameFilter>,
    pub whitelist: FieldFilter,
    pub blacklist: FieldFilter,
    pub exact_whitelist: FieldFilter,
    pub exact_blacklist: FieldFilter,
    pub match_any: bool,
}

#[derive(Debug, Clone)]
pub struct FieldFilter {
    pub generic: Option<Vec<String>>,
    pub library: Option<Vec<String>>,
    pub title: Option<Vec<String>>,
    pub developer: Option<Vec<String>>,
    pub publisher: Option<Vec<String>>,
    pub series: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct ForcedGameFIlter {
    pub whitelist: ForcedFieldFilter,
    pub blacklist: ForcedFieldFilter,
    pub exact_whitelist: ForcedFieldFilter,
    pub exact_blacklist: ForcedFieldFilter,
}


#[derive(Debug, Clone)]
struct ForcedFieldFilter {
    pub generic: Vec<String>,
    pub library: Vec<String>,
    pub title: Vec<String>,
    pub developer: Vec<String>,
    pub publisher: Vec<String>,
    pub series: Vec<String>,
    pub tags: Vec<String>,
    pub platforms: Vec<String>,
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
            offset: None,
            limit: 1000,
            slim: false,
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
        }
    }
}

impl Default for FieldFilter {
    fn default() -> Self {
        FieldFilter {
            generic: None,
            library: None,
            title: None,
            developer: None,
            publisher: None,
            series: None,
            tags: None,
            platforms: None,
        }
    }
}

impl Default for ForcedGameFIlter {
    fn default() -> Self {
        ForcedGameFIlter {
            whitelist: ForcedFieldFilter::default(),
            blacklist: ForcedFieldFilter::default(),
            exact_whitelist: ForcedFieldFilter::default(),
            exact_blacklist: ForcedFieldFilter::default(),
        }
    }
}

impl Default for ForcedFieldFilter {
    fn default() -> Self {
        ForcedFieldFilter {
            generic: vec![],
            library: vec![],
            title: vec![],
            developer: vec![],
            publisher: vec![],
            series: vec![],
            tags: vec![],
            platforms: vec![],
        }
    }
}

impl From<&ForcedGameFIlter> for GameFilter {
    fn from(value: &ForcedGameFIlter) -> Self {
        let mut search = GameFilter::default();

        // Whitelist

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

        // Blacklist

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

        // Exact whitelist

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

        // Exact blacklist


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

        search
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

const RESULTS_QUERY: &str = "SELECT id, title, alternateTitles, series, developer, publisher, platformsStr, \
platformName, dateAdded, dateModified, broken, extreme, playMode, status, notes, \
tagsStr, source, applicationPath, launchCommand, releaseDate, version, \
originalDescription, language, activeDataId, activeDataOnDisk, lastPlayed, playtime, \
activeGameConfigId, activeGameConfigOwner, archiveState, library \
FROM game";

const SLIM_RESULTS_QUERY: &str = "SELECT id, title, series, developer, publisher, platformsStr, 
platformName, tagsStr, library 
FROM game";

pub fn search_index(conn: &Connection, search: &GameSearch) -> Result<Vec<String>> {
    let offset_column = match search.order.column {
        GameSearchSortable::TITLE => "game.title"
    };
    let selection = format!("SELECT id, ROW_NUMBER() OVER (ORDER BY {}, id) AS rn FROM game", offset_column);
    let (mut query, mut params) = build_search_query(search, &selection);
    
    // Add the weirdness
    query = format!("SELECT id FROM ({}) WHERE rn % ? = 0", query);
    params.push(search.limit.to_string());

    let params_as_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let mut ids = vec![];
    let mut stmt = conn.prepare(&query)?;
    let ids_iter = stmt.query_map(params_as_refs.as_slice(), |row| {
        row.get::<_, String>(0)
    })?;
    for i in ids_iter {
        ids.push(i?);
    }
    Ok(ids)
}

pub fn search_count(conn: &Connection, search: &GameSearch) -> Result<i64> {
    let mut countable_search = search.clone();
    // Remove result limit for COUNT queries
    countable_search.limit = 99999999999;
    let (query, params) = build_search_query(search, COUNT_QUERY);

    let params_as_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let count_result = conn.query_row(&query, params_as_refs.as_slice(), |row| {
        row.get::<_, i64>(0)
    }).optional()?;

    match count_result {
        Some(count) => Ok(count),
        None => Ok(0)
    }
}

// The search function that takes a connection and a GameSearch object
pub fn search(conn: &Connection, search: &GameSearch) -> Result<Vec<Game>> {
    let selection = match search.slim {
        true => SLIM_RESULTS_QUERY,
        false => RESULTS_QUERY
    };
    let (query, params) = build_search_query(search, selection);

    // Convert the parameters array to something rusqlite understands
    let params_as_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    debug_println!("{}", format_query(&query, params.clone()));

    let mut games = Vec::new();

    let mut stmt = conn.prepare(query.as_str())?;
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
                detailed_platforms: None,
                detailed_tags: None,
                game_data: None,
            })
        },
    };
    let game_iter = stmt.query_map(params_as_refs.as_slice(), game_map_closure)?;

    for game in game_iter {
        let mut game: Game = game?;
        if search.load_relations.platforms {
            game.detailed_platforms = get_game_platforms(conn, &game.id)?.into();
        }
        if search.load_relations.tags {
            game.detailed_tags = get_game_tags(conn, &game.id)?.into();
        }
        if search.load_relations.game_data {
            game.game_data = Some(get_game_data(conn, &game.id)?);
        }
        games.push(game);
    }

    Ok(games)
}

fn build_search_query(search: &GameSearch, selection: &str) -> (String, Vec<String>) {
    let mut query = String::from(selection);

    // Ordering
    let order_column = match search.order.column {
        GameSearchSortable::TITLE => "game.title"
    };
    let order_direction = match search.order.direction {
        GameSearchDirection::ASC => "ASC",
        GameSearchDirection::DESC => "DESC"
    };

    // Build the inner WHERE clause
    let mut params: Vec<String> = vec![];
    let where_clause = build_filter_query(&search.filter, &mut params);
    if let Some(offset) = search.offset.clone() {
        let offset_clause = match search.order.direction {
            GameSearchDirection::ASC => {
                format!(" WHERE ({} > ? OR ({} = ? AND game.id > ?))", order_column, order_column)
            },
            GameSearchDirection::DESC => {
                format!(" WHERE ({} < ? OR ({} = ? AND game.id < ?))", order_column, order_column)
            }
        };
        query.push_str(&offset_clause);
        // Insert in reverse order
        params.insert(0, offset.game_id.clone());
        params.insert(0, offset.value.clone());
        params.insert(0, offset.value.clone());
    }

    // Combine all where clauses
    if where_clause.len() > 0 {
        // Offset will begin WHERE itself, otherwise we're ANDing the offset
        let start_clause = match search.offset {
            Some(_) => " AND (",
            None => " WHERE ("
        };
        query.push_str(start_clause);
        query.push_str(&where_clause);
        query.push_str(")");
    }

    query.push_str(format!(" ORDER BY {} {}, game.id {}", order_column, order_direction, order_direction).as_str());
    let limit_query = format!(" LIMIT {}", search.limit);
    query.push_str(&limit_query);

    (query, params)
}

fn build_filter_query(filter: &GameFilter, params: &mut Vec<String>) -> String {
    let mut where_clauses = Vec::new();

    if filter.subfilters.len() > 0 {
        for subfilter in filter.subfilters.iter() {
            where_clauses.push(build_filter_query(subfilter, params));
        }
    }

    let mut add_clause = |field_name: &str, values: &Option<Vec<String>>, exact: bool, blacklist: bool| {
        if let Some(value_list) = values {
            let comparator = match (blacklist, exact) {
                (true, true) => "!=",
                (true, false) => "NOT LIKE",
                (false, true) => "=",
                (false, false) => "LIKE",
            };

            for value in value_list {
                where_clauses.push(format!("game.{} {} ?", field_name, comparator));
                if exact {
                    params.push(value.clone());
                } else {
                    let p = format!("%{}%", value);
                    params.push(p);
                }
            }
        }
    };

    // exact whitelist
    exact_whitelist_clause!(add_clause, "library", &filter.exact_whitelist.library);
    exact_whitelist_clause!(add_clause, "developer", &filter.exact_whitelist.developer);
    exact_whitelist_clause!(add_clause, "publisher", &filter.exact_whitelist.publisher);

    // exact blacklist
    exact_blacklist_clause!(add_clause, "library", &filter.exact_blacklist.library);
    exact_blacklist_clause!(add_clause, "developer", &filter.exact_blacklist.developer);
    exact_blacklist_clause!(add_clause, "publisher", &filter.exact_blacklist.publisher);

    // whitelist
    whitelist_clause!(add_clause, "library", &filter.whitelist.library);
    whitelist_clause!(add_clause, "developer", &filter.whitelist.developer);
    whitelist_clause!(add_clause, "publisher", &filter.whitelist.publisher);

    // blacklist
    blacklist_clause!(add_clause, "library", &filter.blacklist.library);
    blacklist_clause!(add_clause, "developer", &filter.blacklist.developer);
    blacklist_clause!(add_clause, "publisher", &filter.blacklist.publisher);

    let mut add_tagged_clause = |tag_name: &str, values: &Option<Vec<String>>, exact: bool, blacklist: bool| {
        if let Some(value_list) = values {
            let comparator = match blacklist {
                true => "NOT IN",
                false => "IN",
            };

            let mut inner_tag_queries = vec![];
            for value in value_list {
                if exact {
                    inner_tag_queries.push("name = ?");
                    params.push(value.clone());
                } else {
                    inner_tag_queries.push("name LIKE ?");
                    let p = format!("%{}%", value);
                    params.push(p);
                }
            }

            let tag_query = match filter.match_any {
                false => {
                    if inner_tag_queries.len() == 1 {
                        format!("game.id {} (SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                            SELECT tagId FROM {}_alias WHERE {})
                        )", comparator, tag_name, tag_name, tag_name, tag_name, inner_tag_queries[0])
                    } else {
                        let mut q = format!("SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                                SELECT tagId FROM {}_alias WHERE {}
                            )", tag_name, tag_name, tag_name, tag_name, inner_tag_queries[0]);
                        for inner_tag_query in inner_tag_queries.iter().skip(1) {
                            let part = format!(" AND gameId IN (
                                SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                                    SELECT tagId FROM {}_alias WHERE {}
                                )
                            )", tag_name, tag_name, tag_name, tag_name, inner_tag_query);
                            q.push_str(&part);
                        }
                        format!("game.id {} ({})", comparator, q)
                    }
                },
                true => format!("game.id {} (SELECT gameId FROM game_{}s_{} WHERE {}Id IN (
                SELECT tagId FROM {}_alias WHERE {}))", comparator, tag_name, tag_name, tag_name, tag_name, inner_tag_queries.join(" OR "))
            };

            where_clauses.push(tag_query);
        }
    };

    // tag groups
    add_tagged_clause("tag", &filter.whitelist.tags, false, false);
    add_tagged_clause("tag", &filter.blacklist.tags, false, true);
    add_tagged_clause("tag", &filter.exact_whitelist.tags, true, false);
    add_tagged_clause("tag", &filter.exact_blacklist.tags, true, true);

    let mut add_multi_clause = |field_names: Vec<&str>, filter: &Option<Vec<String>>, exact: bool, blacklist: bool| {
        if let Some(value_list) = filter {
            let mut multi_where_clauses = vec![];

            let comparator = match (blacklist, exact) {
                (true, true) => "!=",
                (true, false) => "NOT LIKE",
                (false, true) => "=",
                (false, false) => "LIKE",
            };

            for value in value_list {
                for field_name in field_names.clone() {
                    multi_where_clauses.push(format!("game.{} {} ?", field_name, comparator));
                    if exact {
                        params.push(value.clone());
                    } else {
                        let p = format!("%{}%", value);
                        params.push(p);
                    }
                }
            }

            where_clauses.push(format!("({})", &multi_where_clauses.join(" OR ")));
        }
    };

    // whitelist
    add_multi_clause(vec!["title", "alternateTitles"], &filter.whitelist.title, false, false);
    
    // blacklist
    add_multi_clause(vec!["title", "alternateTitles"], &filter.blacklist.title, false, true);

    if filter.match_any {
        return where_clauses.join(" OR ");
    } else {
        return where_clauses.join(" AND ");
    }
}

fn format_query(query: &str, substitutions: Vec<String>) -> String {
    let mut formatted_query = String::new();
    let mut trim_mode = false;
    let mut indent = 0;
    let mut substitution_iter = substitutions.iter();

    for ch in query.chars() {
        match ch {
            '(' => {
                indent += 4;
                trim_mode = true;
                formatted_query.push(ch);
                formatted_query.push('\n');
            },
            ')' => {
                trim_mode = false;
                indent -= 4;
                formatted_query.push('\n');
                let spaces = " ".repeat(indent);
                formatted_query.push_str(&spaces);
                formatted_query.push(ch);
            },
            '?' => {
                if let Some(subst) = substitution_iter.next() {
                    let wrapped_subst = format!("'{}'", subst);
                    formatted_query.push_str(&wrapped_subst);
                } else {
                    // If there are no more substitutions, keep the '?' or handle as needed
                    formatted_query.push(ch);
                }
            },
            ' ' => {
                if !trim_mode {
                    formatted_query.push(ch);
                }
            },
            '\n' => trim_mode = true,
            _ => {
                if trim_mode {
                    let spaces = " ".repeat(indent);
                    formatted_query.push_str(&spaces);
                    trim_mode = false;
                }
                formatted_query.push(ch);
            },
        }
    }

    formatted_query
}

pub fn parse_user_input(input: &str) -> GameSearch {
    let mut search = GameSearch::default();
    let mut filter = ForcedGameFIlter::default();

    let mut capturing_quotes = false;
    let mut working_key = String::new();
    let mut working_value = String::new();
    let mut negative = false;
    let mut exact = false;

    for mut token in input.split_whitespace() {
        // Value on the same scope as token to append to
        let mut _t = "".to_owned();
        debug_println!("token {}", token);
        // Handle continued value capture if needed

        if !capturing_quotes && token.len() > 1 {
            // Not inside quotes, check for negation
            if token.starts_with("-") {
                negative = true;

                token = token.strip_prefix("-").unwrap();
            }

            if token.len() > 1 {
                let ch = token.chars().next().unwrap();
                if ch == '=' {
                    token = token.strip_prefix('=').unwrap();
                    exact = true;
                }
            }

            if token.len() > 1 {
                debug_println!("checking token start");
                // Check for quick search options preceding token
                let ch = token.chars().next().unwrap();
                debug_println!("start char: {}", ch);
                match ch {
                    '#' => {
                        token = token.strip_prefix('#').unwrap();
                        working_key = "tag".to_owned();
                    },
                    '!' => {
                        token = token.strip_prefix('!').unwrap();
                        working_key = "platform".to_owned();
                    },
                    '@' => {
                        token = token.strip_prefix('@').unwrap();
                        working_key = "developer".to_owned();
                    }
                    _ => {
                        // No special token, check if we're preceding a key
                        if !token.contains(':') && exact{
                            // No key, is generic, do not use exact
                            exact = false;
                            _t = "=".to_owned() + token;
                            token = &_t;
                        }
                    }
                }
            }

        }

        if token.starts_with('"') {
            token = token.strip_prefix('"').unwrap();
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

        if token.ends_with('"') {
            // Closing quote
            capturing_quotes = false;
            // Remove quote at end of working value, if doesn't exist then it's a broken quoted value
            working_value = working_value.strip_suffix('"').unwrap().to_owned();
        }

        if capturing_quotes {
            // Still in capture mode, get next token
            continue;
        }

        if working_value == "" {
            // No working input yet, check for key
            let token_parts = token.split(":").collect::<Vec<&str>>();

            if token_parts.len() > 1 {
                // Has a key
                working_key = token_parts[0].to_owned();
                token = token_parts[1];
            } else {
                token = token_parts[0];
            }

            // Single value, must be value
            if token.starts_with('"') && token.ends_with('"') {
                // Fully inside quotes
                token = token.strip_prefix('"').unwrap();
                token = token.strip_suffix('"').unwrap();
                working_value = token.to_owned();
            } else {
                if token.starts_with('"') {
                    // Starts quotes
                    token = token.strip_prefix('"').unwrap();
                    capturing_quotes = true;
                    working_value = token.to_owned();
                    continue;
                } else {
                    // Not quoted
                    working_value = token.to_owned();
                }
            }
        }

        if working_value != "" {
            debug_println!("key: {}, value: {}, negative: {}", working_key, working_value, negative);

            let mut list = match (negative, exact) {
                (true, false) => filter.blacklist.clone(),
                (false, false) => filter.whitelist.clone(),
                (true, true) => filter.exact_blacklist.clone(),
                (false, true) => filter.exact_whitelist.clone(),
            };
            let value = working_value.clone();

            // Has a complete value, add to filter
            match working_key.as_str() {
                "library" => list.library.push(value),
                "title" => list.title.push(value),
                "developer" => list.developer.push(value),
                "publisher" => list.publisher.push(value),
                "series" => list.series.push(value),
                "tag" => list.tags.push(value),
                "platform" => list.platforms.push(value),
                _ => list.generic.push(value),
            }

            match (negative, exact) {
                (true, false) => filter.blacklist = list,
                (false, false) => filter.whitelist = list,
                (true, true) => filter.exact_blacklist = list,
                (false, true) => filter.exact_whitelist = list,
            }

            negative = false;
            exact = false;
            working_value.clear();
            working_key.clear();
        }
    }

    search.filter = (&filter).into();

    search
}