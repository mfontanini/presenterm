use clap::Parser;
use comrak::{Arena, ComrakOptions};
use presenterm::{draw::Drawer, highlighting::CodeHighlighter, parse::SlideParser, resource::Resources};
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

    let resources = Resources::default();
    let highlighter = CodeHighlighter::new("Solarized (light)").expect("creating highlighter failed");
    let mut drawer = Drawer::new(resources, highlighter).expect("creating drawer failed");
    drawer.draw(&slides).expect("draw failed");
    std::thread::sleep(std::time::Duration::from_secs(10));
}
