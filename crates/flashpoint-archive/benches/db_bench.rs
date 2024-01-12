use std::{fs::File, io::{BufReader, BufRead}, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use flashpoint_archive::Flashpoint;


pub fn criterion_benchmark(c: &mut Criterion) {
    let flashpoint = Flashpoint::new();
    flashpoint.load_database("flashpoint.sqlite").expect("Failed to open database");
    let file = File::open("benches/1k_rand.txt").expect("Failed to open file");
    let reader = BufReader::new(file);
    let mut game_ids: Vec<String> = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(line_content) => game_ids.push(line_content),
            Err(err) => eprintln!("Error reading line: {}", err),
        }
    }

    // Benchmark the find_game function using the game_ids
    let mut group = c.benchmark_group("1k-find");
    group.sample_size(10).measurement_time(Duration::from_secs(30));
    group.bench_function("find 1k", |b| {
        b.iter(|| {
            for id in &game_ids {
                let _game = flashpoint.find_game(id).expect("Failed to load game");
                // Do something with the game if needed
            }
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
