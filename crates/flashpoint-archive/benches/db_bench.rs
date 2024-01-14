use std::{fs::File, io::{BufReader, BufRead}, time::Duration};
use std::fs;
use criterion::{Criterion, criterion_group, criterion_main};
use flashpoint_archive::Flashpoint;
use flashpoint_archive::game::search::GameSearch;


pub fn criterion_benchmark(c: &mut Criterion) {
    let source_file = "flashpoint.sqlite"; // Replace with the path to your source file
    let destination_file = "/mnt/tmp/flashpoint.sqlite"; // Replace with the path to your destination file

    // Copy the file from source to destination
    fs::copy(source_file, destination_file).expect("Failed to set up database");

    let flashpoint = Flashpoint::new();
    flashpoint.load_database(source_file).expect("Failed to open database");
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

    // Benchmark the find_game function using the game_ids
    let mut group = c.benchmark_group("1k-find");
    group.sample_size(10).measurement_time(Duration::from_secs(35));
    group.bench_function("find 1k", |b| {
        b.iter(|| {
            for id in &rand_game_ids {
                flashpoint.find_game(id).expect("Failed to load game");
            }
        })
    });

    group.bench_function("full scan", |b| {
        b.iter(|| {
            let mut search = GameSearch::default();
            search.limit = 9999999;
            search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
            flashpoint.search_games(&search).expect("Failed to search");
        })
    });

    group.bench_function("full scan with tag filter groups", |b| {
        b.iter(|| {
            let mut search = GameSearch::default();
            search.limit = 999999999;
            search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
            search.filter.exact_blacklist.tags = Some(vec![]);
            flashpoint.search_games(&search).expect("Failed to search");
        })
    });


    group.bench_function("search 15", |b| {
        b.iter(|| {
            for search_term in &search_terms {
                let mut search = GameSearch::default();
                search.filter.whitelist.title = Some(vec![search_term.clone()]);
                search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
                flashpoint.search_games(&search).expect("Failed to search");
            }
        })
    });

    group.bench_function("search 15 uncapped", |b| {
        b.iter(|| {
            for search_term in &search_terms {
                let mut search = GameSearch::default();
                search.limit = 999999999;
                search.filter.whitelist.title = Some(vec![search_term.clone()]);
                search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
                flashpoint.search_games(&search).expect("Failed to search");
            }
        })
    });

    group.bench_function("search 15 with relations", |b| {
        b.iter(|| {
            for search_term in &search_terms {
                let mut search = GameSearch::default();
                search.load_relations.tags = true;
                search.load_relations.platforms = true;
                search.load_relations.game_data = true;
                search.filter.whitelist.title = Some(vec![search_term.clone()]);
                search.filter.exact_whitelist.library = Some(vec![String::from("arcade")]);
                flashpoint.search_games(&search).expect("Failed to search");
            }
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
