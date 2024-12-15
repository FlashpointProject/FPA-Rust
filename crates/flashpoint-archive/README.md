# Flashpoint Archive Rust Library

Rust library for accessing the local database and misc features of the Flashpoint Archive.

Project website: https://flashpointarchive.org/

## Feature flags

`napi` - Adds napi-rs attrs to structs

## Test

- Copy test db from https://download.unstable.life/testing/flashpoint.sqlite to `crates/flashpoint-archive/benches`
`cargo test -p flashpoint-archive`

## Build

`cargo build -p flashpoint-archive`

Node binding:
`cd bindings/binding-node`
`npm install`
- Swap for path in cargo.toml - `flashpoint-archive = { path = "../../crates/flashpoint-archive", features = ["napi"] }`
`npm run build`

Dev node binding:
`cd bindings/binding-node`
`npm link`
`cd <launcher-dev-folder>`
`npm link @fparchive/flashpoint-archive`

## Usage

### User input example

```rust
use flashpoint_archive::FlashpointArchive;
use flashpoint_archive::games::search::parse_user_input;

fn main() {
    let archive = FlashpointArchive::new();
    archive.load_database(TEST_DATABASE).expect("Failed to open database");

    let mut search = parse_user_input("Sonic platform:Flash");
    search.limit = 9999999; // Default 1000 limit for pages
    let games = archive.find_games(search).expect("Failed to search");
}
```