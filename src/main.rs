use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    input::source::CommandSource,
    markdown::parse::MarkdownParser,
    render::highlighting::CodeHighlighter,
    resource::Resources,
    slideshow::{SlideShow, SlideShowMode},
    theme::PresentationTheme,
};
use std::path::{Path, PathBuf};

/// Run slideshows from your terminal.
#[derive(Parser)]
struct Cli {
    /// The path to the markdown file that contains the presentation.
    path: PathBuf,

    /// Whether to use presentation mode.
    #[clap(short, long, default_value_t = false)]
    present: bool,

    /// The theme to use.
    #[clap(short, long, default_value = "dark")]
    theme: String,
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let Some(default_theme) = PresentationTheme::from_name(&cli.theme) else {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::InvalidValue, "invalid theme name").exit();
    };

    let mode = match cli.present {
        true => SlideShowMode::Presentation,
        false => SlideShowMode::Development,
    };
    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    let default_highlighter = CodeHighlighter::new("base16-ocean.dark")?;
    let resources_path = cli.path.parent().unwrap_or(Path::new("/"));
    let resources = Resources::new(resources_path);
    let commands = CommandSource::new(&cli.path);

    let slideshow = SlideShow::new(&default_theme, default_highlighter, commands, parser, resources, mode);
    slideshow.present(&cli.path)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Failed to run presentation: {e}");
    }
}
