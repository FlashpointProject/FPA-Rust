use std::{fs::File, io::{BufReader, BufRead}, time::Duration};
use criterion::{Criterion, criterion_group, criterion_main};
use flashpoint_archive::{FlashpointArchive, game::search::GameFilter};
use flashpoint_archive::game::search::GameSearch;
use tokio::runtime::Runtime;

const TEST_DATABASE: &str = "benches/flashpoint.sqlite";

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut flashpoint = FlashpointArchive::new();
    flashpoint.load_database(TEST_DATABASE).expect("Failed to open database");
    let rand_file = File::open("benches/1k_rand.txt").expect("Failed to open file");
    let rand_reader = BufReader::new(rand_file);
    let mut rand_game_ids = vec![];
    for line in rand_reader.lines() {
        match line {
            Ok(line_content) => rand_game_ids.push(line_content),
            Err(err) => eprintln!("Error reading line: {}", err),
        }
    }

    let search_file = File::open("benches/15_search.txt").expect("Failed to open file");
    let search_reader = BufReader::new(search_file);
    let mut search_terms = vec![];
    for line in search_reader.lines() {
        match line {
            Ok(line_content) => search_terms.push(line_content),
            Err(err) => eprintln!("Error reading line: {}", err),
        }
    }

    let blacklist_file = File::open("benches/tags_blacklist.txt").expect("Failed to open file");
    let blacklist_reader = BufReader::new(blacklist_file);
    let mut blacklist_tags = vec![];
    for line in blacklist_reader.lines() {
        match line {
            Ok(line_content) => blacklist_tags.push(line_content),
            Err(err) => eprintln!("Error reading line: {}", err),
        }
    }


    // Benchmark the find_game function using the game_ids
    let mut group = c.benchmark_group("benches");
    group.sample_size(10).measurement_time(Duration::from_secs(35));
    group.bench_function("find 1k", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            for id in &rand_game_ids {
                flashpoint.find_game(id).await.expect("Failed to load game");
            }
        })
    });

    group.bench_function("full scan", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            let mut search = GameSearch::default();
            search.limit = 99999999999;
            search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
            flashpoint.search_games(&search).await.expect("Failed to search");
        })
    });

    group.bench_function("full scan with unoptimized tag filter groups", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            let mut search = GameSearch::default();
            search.limit = 99999999999;
            let mut tag_filter = GameFilter::default();
            tag_filter.exact_blacklist.tags = Some(blacklist_tags.clone());
            tag_filter.match_any = true;
            search.filter.subfilters.push(tag_filter);
            search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
            flashpoint.search_games(&search).await.expect("Failed to search");
        })
    });

    group.bench_function("search 15", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            for search_term in &search_terms {
                let mut search = GameSearch::default();
                search.filter.whitelist.title = Some(vec![search_term.clone()]);
                search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
                flashpoint.search_games(&search).await.expect("Failed to search");
            }
        })
    });

    group.bench_function("search 15 uncapped", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async{
            for search_term in &search_terms {
                let mut search = GameSearch::default();
                search.limit = 99999999999;
                search.filter.whitelist.title = Some(vec![search_term.clone()]);
                search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
                flashpoint.search_games(&search).await.expect("Failed to search");
            }
        })
    });

    group.bench_function("search 15 with relations", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            for search_term in &search_terms {
                let mut search = GameSearch::default();
                search.load_relations.tags = true;
                search.load_relations.platforms = true;
                search.load_relations.game_data = true;
                search.filter.whitelist.title = Some(vec![search_term.clone()]);
                search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
                flashpoint.search_games(&search).await.expect("Failed to search");
            }
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
