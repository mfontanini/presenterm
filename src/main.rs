use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    input::source::CommandSource, markdown::parse::MarkdownParser, render::highlighting::CodeHighlighter,
    resource::Resources, slideshow::SlideShow, theme::PresentationTheme,
};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    path: PathBuf,

    #[clap(default_value = "dark")]
    theme: String,
}

fn main() {
    let cli = Cli::parse();
    let Some(theme) = PresentationTheme::from_name(&cli.theme) else {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::InvalidValue, "invalid theme name").exit();
    };

    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    let highlighter = CodeHighlighter::new("base16-ocean.dark").expect("creating highlighter failed");
    let resources = Resources::new(cli.path.parent().expect("no parent"));
    let commands = CommandSource::new(&cli.path);

    let slideshow = SlideShow::new(theme, commands, parser, resources, highlighter);
    if let Err(e) = slideshow.present(&cli.path) {
        eprintln!("Error running slideshow: {e}");
    };
}
