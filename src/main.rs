use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    input::source::CommandSource,
    markdown::parse::MarkdownParser,
    presenter::{PresentMode, Presenter},
    render::highlighting::CodeHighlighter,
    resource::Resources,
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
        let valid_themes = PresentationTheme::theme_names().collect::<Vec<_>>().join(", ");
        let error_message = format!("invalid theme name, valid themes are: {valid_themes}");
        cmd.error(ErrorKind::InvalidValue, error_message).exit();
    };

    let mode = match cli.present {
        true => PresentMode::Presentation,
        false => PresentMode::Development,
    };
    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    let default_highlighter = CodeHighlighter::new("base16-ocean.dark")?;
    let resources_path = cli.path.parent().unwrap_or(Path::new("/"));
    let resources = Resources::new(resources_path);
    let commands = CommandSource::new(&cli.path);

    let presenter = Presenter::new(&default_theme, default_highlighter, commands, parser, resources, mode);
    presenter.present(&cli.path)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Failed to run presentation: {e}");
    }
}
