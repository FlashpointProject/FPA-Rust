#![allow(non_snake_case)]

use std::error::Error;
use std::fs;

use clap::{command, Parser};
use flashpoint_archive::game::search::{GameSearchDirection, GameSearchSortable};
use flashpoint_archive::{update::RemoteCategory, FlashpointArchive};
use flashpoint_archive::update::{RemoteGamesRes, RemotePlatform, RemoteTag};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// FPFSS url, without ending slash
    #[arg(long, default_value_t = String::from("https://fpfss.unstable.life"))]
    url: String,

    /// Update the existing database instead of replacing it with a new one
    #[arg(short, long, action)]
    update: bool,

    #[arg(short, long, default_value_t = String::from("./flashpoint.sqlite"))]
    database: String,
}

#[tokio::main]
async fn main() {
    // Parse arguments
    let args = Args::parse();

    // Delete database if exists
    if !args.update && fs::metadata(&args.database).is_ok() {
        fs::remove_file(&args.database).expect("Failed to delete existing database");
    }

    // Open database
    let mut fp = FlashpointArchive::new();
    fp.load_database(&args.database).expect("Failed to load database");

    let mut search = fp.parse_user_input("").search;
    search.order.column = GameSearchSortable::DATEMODIFIED;
    search.order.direction = GameSearchDirection::DESC;

    // Find last date, use for avoiding redundant updates
    let existing_games = fp.search_games(&search).await.expect("Failed to search db");
    let last_date = if existing_games.len() > 0 {
        &existing_games.get(0).unwrap().date_modified
    } else {
        "1970-01-01"
    };

    if args.update {
        println!("Update Mode: Latest date {}", last_date);
    }

    let updates_ready = fetch_update_info(&args.url, last_date).await.expect("Failed to check update count");

    println!("Fetching {} game updates...", updates_ready);

    let plats = fetch_platforms(&args.url).await.expect("Failed to search platforms");
    println!("Applying {} platforms", plats.len());
    fp.update_apply_platforms(plats).await.expect("Failed to update platforms in database");

    let tags_res = fetch_tags(&args.url).await.expect("Failed to search tags and categories");
    println!("Applying {} categories", tags_res.categories.len());
    fp.update_apply_categories(tags_res.categories).await.expect("Failed to update categories in database");
    println!("Applying {} tags", tags_res.tags.len());
    fp.update_apply_tags(tags_res.tags.iter().map::<RemoteTag, _>(|t| RemoteTag {
        id: t.id, 
        name: t.name.clone(), 
        description: t.description.clone(), 
        category: t.category.clone(), 
        date_modified: t.date_modified.clone(), 
        aliases: t.aliases.split(';').into_iter().map(|a| a.trim().to_owned()).collect(), 
        deleted: t.Deleted
    }).collect()).await.expect("Failed to update tags in database");

    let mut total_applied_games = 0;
    let mut page_num = 1;
    let mut next_id = None;
    loop {
        println!("Fetching page {}", page_num);
        let res = fetch_games(&args.url, last_date, next_id.clone()).await.expect("Failed to fetch games page");
        page_num += 1;
        if res.games.len() > 0 {
            total_applied_games += res.games.len();
            next_id = Some(res.games.last().unwrap().id.clone());
            fp.update_apply_games(&res, "flashpoint-archive").await.expect("Failed to apply game page update");
        } else {
            break;
        }
    }

    println!("Applied {} games", total_applied_games);
}

async fn fetch_platforms(base_url: &str) -> Result<Vec<RemotePlatform>, Box<dyn Error>> {
    let plat_url = format!(
        "{}/api/platforms",
        base_url
    );

    let res = reqwest::get(&plat_url)
        .await?
        .json::<Vec<RemotePlatformRaw>>()
        .await?;

    Ok(res.iter().map::<RemotePlatform, _>(|r| RemotePlatform {
        id: r.id,
        name: r.name.clone(),
        description: r.description.clone(),
        date_modified: r.date_modified.clone(),
        aliases: r.aliases.split(';').into_iter().map(|a| a.trim().to_owned()).collect(),
        deleted: r.Deleted,
    }).collect())
}

async fn fetch_tags(base_url: &str) -> Result<RemoteTagRes, Box<dyn Error>> {
    let tags_url = format!(
        "{}/api/tags",
        base_url
    );

    let res = reqwest::get(&tags_url)
        .await?
        .json::<RemoteTagRes>()
        .await?;

    Ok(res)
}

async fn fetch_games(base_url: &str, last_date: &str, last_id: Option<String>) -> Result<RemoteGamesRes, Box<dyn Error>> {
    let mut games_url = format!(
        "{}/api/games?broad=true&after={}",
        base_url,
        last_date
    );

    if let Some(id) = last_id {
        games_url.push_str(format!("&afterId={}", id).as_str());
    }

    let resp = reqwest::get(&games_url)
        .await?
        .json::<RemoteGamesRes>()
        .await?;

    Ok(resp)
}

async fn fetch_update_info(base_url: &str, last_date: &str) -> Result<i64, Box<dyn Error>> {
    let count_url = format!(
        "{}/api/games/updates?after={}",
        base_url,
        last_date
    );

    let resp = reqwest::get(&count_url)
        .await?
        .json::<UpdateInfo>()
        .await?;

    Ok(resp.total)
}

#[derive(Deserialize, Serialize)]
struct UpdateInfo {
    total: i64
}

#[derive(Debug, Deserialize)]
struct RemotePlatformRaw {
    id: i64,
    name: String,
    description: String,
    date_modified: String,
    aliases: String,
    Deleted: bool,
}

#[derive(Debug, Deserialize)]
struct RemoteTagRes {
    tags: Vec<RemoteTagRaw>,
    categories: Vec<RemoteCategory>,
}

#[derive(Debug, Deserialize)]
struct RemoteTagRaw {
    id: i64,
    name: String,
    description: String,
    date_modified: String,
    category: String,
    aliases: String,
    Deleted: bool,
}
