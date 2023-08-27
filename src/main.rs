use clap::Parser;
use comrak::{Arena, ComrakOptions};
use presenterm::parse::SlideParser;
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Cli {
    path: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let parser = SlideParser::new(&arena, options);

    let content = fs::read_to_string(cli.path).expect("reading failed");
    let slides = parser.parse(&content).expect("parse failed");
    for slide in slides {
        println!("{slide:?}");
    }
}
