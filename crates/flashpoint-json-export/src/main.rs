#![allow(non_snake_case)]

use clap::{command, Parser};

use flashpoint_archive::{FlashpointArchive, game::search::GameSearch};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("./flashpoint.sqlite"))]
    database: String,
    #[arg(short, long, default_value_t = String::from("./export.json"))]
    output: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Open database
    let mut fp = FlashpointArchive::new();
    fp.load_database(&args.database).expect("Failed to load database");

    let mut dump = LauncherDump { 
        games: LauncherDumpGames {
            add_apps: vec![],
            game_data: vec![],
            games: vec![],
        }, 
        tags: LauncherDumpTags {
            aliases: vec![],
            categories: vec![],
            tags: vec![],
        }, 
        platforms: LauncherDumpPlatforms {
            aliases: vec![],
            platforms: vec![],
        }, 
        tag_relations: vec![], 
        platform_relations: vec![]
    };

    // Load all Platforms
    println!("Collecting platforms...");
    let platforms = fp.find_all_platforms().await.expect("Failed to read platforms");
    
    let platform_aliases: Vec<PlatformAlias> = platforms
        .iter()
        .flat_map(|p| {
            p.aliases.iter().map(move |alias| PlatformAlias {
                platform_id: p.id,
                name: alias.clone(),
            })
        })
        .collect();

    dump.platforms = LauncherDumpPlatforms {
        aliases: platform_aliases,
        platforms: platforms.into_iter().map(|p| LauncherDumpPlatformsPlatform {
            id: p.id,
            description: p.description,
            primary_alias: p.name,
        }).collect(),
    };

    // Load all Tags
    println!("Collecting tags...");
    let tags = fp.find_all_tags().await.expect("Failed to read tags");
    let tagCategories = fp.find_all_tag_categories().await.expect("Failed to read tag categories");

    let tag_aliases: Vec<TagAlias> = tags
        .iter()
        .flat_map(|t| {
            t.aliases.iter().map(move |alias| TagAlias {
                tag_id: t.id,
                name: alias.clone(),
            })
        })
    .collect();

    let category_map: std::collections::HashMap<String, i64> = tagCategories
        .iter()
        .map(|tc| (tc.name.clone(), tc.id))
        .collect();

    dump.tags = LauncherDumpTags {
        aliases: tag_aliases,
        categories: tagCategories.into_iter().map(|tc| TagCategory {
            id: tc.id,
            name: tc.name,
            color: tc.color,
            description: tc.description.unwrap_or_default() 
        }).collect(),
        tags: tags.into_iter().map(|t| LauncherDumpTagsTag {
            id: t.id,
            category_id: t.category
                .as_ref()
                .and_then(|cat| category_map.get(cat))
                .copied()
                .unwrap_or(0),
                description: t.description,
                primary_alias: t.name
        }).collect()
    };

    // Load all Games
    println!("Collecting games...");
    let mut search = GameSearch::default();
    search.limit = 9999999999;
    let games = fp.search_games(&search).await.expect("Failed to read games");

    // Collect all additional apps and game data
    let mut all_add_apps = Vec::new();
    let mut all_game_data = Vec::new();
    let mut tag_relations = Vec::new();
    let mut platform_relations = Vec::new();

    dump.games.games = games.into_iter().map(|g| {
        // Collect additional apps for this game
        if let Some(add_apps) = &g.add_apps {
            for app in add_apps {
                all_add_apps.push(AdditionalApp {
                    id: Some(app.id.clone()),
                    application_path: app.application_path.clone(),
                    auto_run_before: app.auto_run_before,
                    launch_command: app.launch_command.clone(),
                    name: app.name.clone(),
                    wait_for_exit: app.wait_for_exit,
                    parent_game_id: g.id.clone(),
                });
            }
        }

        // Collect game data for this game
        if let Some(data) = g.game_data {
            for gd in data {
                all_game_data.push(GameData {
                    id: gd.id,
                    game_id: gd.game_id,
                    title: gd.title,
                    date_added: gd.date_added,
                    sha_256: gd.sha256,
                    crc_32: gd.crc32,
                    size: gd.size,
                    parameters: gd.parameters,
                    application_path: gd.application_path,
                    launch_command: gd.launch_command,
                    indexed: false,
                    index_error: false,
                });
            }
        }

        // Collect tag relations
        for tag in g.tags {
            tag_relations.push(LauncherDumpRelation {
                game_id: g.id.clone(),
                value: tag,
            });
        }

        // Collect platform relations
        for platform in g.platforms {
            platform_relations.push(LauncherDumpRelation {
                game_id: g.id.clone(),
                value: platform,
            });
        }
    
        GameDump {
            id: g.id,
            title: g.title,
            alternate_titles: g.alternate_titles,
            series: g.series,
            developer: g.developer,
            publisher: g.publisher,
            primary_platform: g.primary_platform,
            date_added: g.date_added.to_string(),
            date_modified: g.date_modified.to_string(),
            play_mode: g.play_mode,
            status: g.status,
            notes: g.notes,
            source: g.source,
            application_path: g.legacy_application_path,
            launch_command: g.legacy_launch_command,
            release_date: g.release_date,
            version: g.version,
            original_desc: g.original_description,
            language: g.language,
            library: g.library,
            active_data_id: g.active_data_id,
            ruffle_support: None,
            action: String::new(),
            reason: String::new(),
            deleted: false,
            user_id: 0,
        }
    }).collect();

    let json = serde_json::to_string_pretty(&dump).expect("Failed to serialize dump");
    std::fs::write(&args.output, json).expect("Failed to write output file");
    println!("Export written to {}", &args.output);
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpRelation {
    #[serde(rename = "g")]
    pub game_id: String,
    #[serde(rename = "v")]
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDump {
    pub games: LauncherDumpGames,
    pub tags: LauncherDumpTags,
    pub platforms: LauncherDumpPlatforms,
    pub tag_relations: Vec<LauncherDumpRelation>,
    pub platform_relations: Vec<LauncherDumpRelation>,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpGames {
    pub add_apps: Vec<AdditionalApp>,
    pub game_data: Vec<GameData>,
    pub games: Vec<GameDump>,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpTags {
    pub categories: Vec<TagCategory>,
    pub aliases: Vec<TagAlias>,
    pub tags: Vec<LauncherDumpTagsTag>,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpPlatforms {
    pub aliases: Vec<PlatformAlias>,
    pub platforms: Vec<LauncherDumpPlatformsPlatform>,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpTagsAliases {
    #[serde(rename = "tagId")]
    pub tag_id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpTagsTag {
    pub id: i64,
    pub category_id: i64,
    pub description: String,
    pub primary_alias: String,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherDumpPlatformsPlatform {
    pub id: i64,
    pub description: String,
    pub primary_alias: String,
}

#[derive(Serialize, Deserialize)]
pub struct GameDump {
    pub id: String,
    pub title: String,
    pub alternate_titles: String,
    pub series: String,
    pub developer: String,
    pub publisher: String,
    pub primary_platform: String,
    pub date_added: String,
    pub date_modified: String,
    pub play_mode: String,
    pub status: String,
    pub notes: String,
    pub source: String,
    #[serde(rename = "legacy_application_path")]
    pub application_path: String,
    #[serde(rename = "legacy_launch_command")]
    pub launch_command: String,
    pub release_date: String,
    pub version: String,
    #[serde(rename = "original_description")]
    pub original_desc: String,
    pub language: String,
    pub library: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_data_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruffle_support: Option<String>,
    pub action: String,
    pub reason: String,
    #[serde(skip)]
    pub deleted: bool,
    #[serde(skip)]
    pub user_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct GameData {
    pub id: i64,
    pub game_id: String,
    pub title: String,
    pub date_added: String,
    pub sha_256: String,
    pub crc_32: i32,
    pub size: i64,
    pub parameters: Option<String>,
    pub application_path: String,
    pub launch_command: String,
    pub indexed: bool,
    pub index_error: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AdditionalApp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub application_path: String,
    pub auto_run_before: bool,
    pub launch_command: String,
    pub name: String,
    pub wait_for_exit: bool,
    pub parent_game_id: String,
}


#[derive(Serialize, Deserialize)]
pub struct TagAlias {
    pub tag_id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlatformAlias {
    pub platform_id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct TagCategory {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: String,
}