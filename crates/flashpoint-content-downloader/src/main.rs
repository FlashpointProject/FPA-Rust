use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// URL to the remote Images directory
    #[arg(long, default_value_t = String::from("https://infinity.unstable.life/images"))]
    images_url: String,

    /// URL to the remote Games directory
    #[arg(long, default_value_t = String::from("https://download.unstable.life/gib-roms/Games"))]
    games_url: String,

    /// Path to flashpoint.sqlite
    #[arg(short, long, default_value_t = String::from("./flashpoint.sqlite"))]
    database: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
}