use rusqlite::{Connection, Result};

use super::{Game, get_game_platforms, get_game_tags, get_game_data};

pub struct GameSearch {
    pub filter: GameFilter,
    pub load_relations: GameSearchRelations,
    pub limit: i64,
    pub slim: bool,
}

pub struct GameSearchRelations {
    pub tags: bool,
    pub platforms: bool,
    pub game_data: bool,
}

pub struct GameFilter {
    pub subfilters: Vec<GameFilter>,
    pub whitelist: FieldFilter,
    pub blacklist: FieldFilter,
    pub exact_whitelist: FieldFilter,
    pub exact_blacklist: FieldFilter,
    pub match_any: bool,
}

pub struct FieldFilter {
    pub generic: Option<Vec<String>>,
    pub library: Option<Vec<String>>,
    pub title: Option<Vec<String>>,
    pub developer: Option<Vec<String>>,
    pub publisher: Option<Vec<String>>,
    pub series: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl Default for GameSearch {
    fn default() -> Self {
        GameSearch {
            filter: GameFilter::default(),
            load_relations: GameSearchRelations::default(),
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
        }
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

// The search function that takes a connection and a GameSearch object
pub fn search(conn: &Connection, search: &GameSearch) -> Result<Vec<Game>> {
    let mut query = match search.slim {
        true =>  String::from("SELECT id, title, series, developer, publisher, platformsStr, 
        platformName, tagsStr, library 
        FROM game"),
        false => String::from("SELECT id, title, alternateTitles, series, developer, publisher, platformsStr, \
        platformName, dateAdded, dateModified, broken, extreme, playMode, status, notes, \
        tagsStr, source, applicationPath, launchCommand, releaseDate, version, \
        originalDescription, language, activeDataId, activeDataOnDisk, lastPlayed, playtime, \
        activeGameConfigId, activeGameConfigOwner, archiveState, library \
        FROM game")
    };

    // Build the inner WHERE clause
    let mut params: Vec<String> = vec![];
    let where_clause = build_filter_query(&search.filter, &mut params);
    // Convert the parameters array to something rusqlite understands
    let params_as_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    // Combine all where clauses
    if where_clause.len() > 0 {
        query.push_str(" WHERE ");
        query.push_str(&where_clause);
    }

    // Ordering
    query.push_str(" ORDER BY game.title ASC");
    let limit_query = format!(" LIMIT {}", search.limit);
    query.push_str(&limit_query);

    println!("{}", format_query(&query, params.clone()));

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