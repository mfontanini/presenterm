use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    CodeHighlighter, CommandSource, Exporter, LoadThemeError, MarkdownParser, PresentMode, PresentationThemeSet,
    Presenter, Resources, Themes,
};
use std::{
    env,
    path::{Path, PathBuf},
};

/// Run slideshows from your terminal.
#[derive(Parser)]
#[command()]
#[command(author, version, about = create_splash(), long_about = create_splash(), arg_required_else_help = true)]
struct Cli {
    /// The path to the markdown file that contains the presentation.
    #[clap(group = "target")]
    path: Option<PathBuf>,

    /// Export the presentation as a PDF rather than displaying it.
    #[clap(short, long)]
    export_pdf: bool,

    /// Generate the PDF metadata without generating the PDF itself.
    #[clap(long, hide = true)]
    generate_pdf_metadata: bool,

    /// Run in export mode.
    #[clap(long, hide = true)]
    export: bool,

    /// Whether to use presentation mode.
    #[clap(short, long, default_value_t = false)]
    present: bool,

    /// The theme to use.
    #[clap(short, long, default_value = "dark")]
    theme: String,

    /// Display acknowledgements.
    #[clap(long, group = "target")]
    acknowledgements: bool,
}

fn create_splash() -> String {
    let crate_version = env!("CARGO_PKG_VERSION");

    format!(
        r#"
  ┌─┐┬─┐┌─┐┌─┐┌─┐┌┐┌┌┬┐┌─┐┬─┐┌┬┐
  ├─┘├┬┘├┤ └─┐├┤ │││ │ ├┤ ├┬┘│││
  ┴  ┴└─└─┘└─┘└─┘┘└┘ ┴ └─┘┴└─┴ ┴ v{}
    A terminal slideshow tool 
                    @mfontanini/presenterm
"#,
        crate_version,
    )
}

fn load_themes() -> Result<Themes, Box<dyn std::error::Error>> {
    let Ok(home) = env::var("HOME") else {
        return Ok(Themes::default());
    };
    let config_path = PathBuf::from(home).join(".config/presenterm");
    let themes_path = config_path.join("themes");
    CodeHighlighter::register_themes_from_path(&themes_path.join("highlighting"))?;
    let mut presentation_themes = PresentationThemeSet::default();

    let register_result = presentation_themes.register_from_directory(&themes_path);
    if let Err(e @ (LoadThemeError::Duplicate(_) | LoadThemeError::Corrupted(..))) = register_result {
        return Err(e.into());
    }
    let themes = Themes { presentation: presentation_themes };
    Ok(themes)
}

fn display_acknowledgements() {
    let acknowledgements = include_bytes!("../bat/acknowledgements.txt");
    println!("{}", String::from_utf8_lossy(acknowledgements));
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let themes = load_themes()?;

    let Some(default_theme) = themes.presentation.load_by_name(&cli.theme) else {
        let mut cmd = Cli::command();
        let valid_themes = themes.presentation.theme_names().join(", ");
        let error_message = format!("invalid theme name, valid themes are: {valid_themes}");
        cmd.error(ErrorKind::InvalidValue, error_message).exit();
    };

    let mode = match (cli.present, cli.export) {
        (true, _) => PresentMode::Presentation,
        (false, true) => PresentMode::Export,
        (false, false) => PresentMode::Development,
    };
    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    let default_highlighter = CodeHighlighter::default();
    if cli.acknowledgements {
        display_acknowledgements();
        return Ok(());
    }
    let path = cli.path.expect("no path");
    let resources_path = path.parent().unwrap_or(Path::new("/"));
    let resources = Resources::new(resources_path);
    if cli.export_pdf || cli.generate_pdf_metadata {
        let mut exporter = Exporter::new(parser, &default_theme, default_highlighter, resources, themes);
        if cli.export_pdf {
            exporter.export_pdf(&path)?;
        } else {
            let meta = exporter.generate_metadata(&path)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    } else {
        let commands = CommandSource::new(&path);
        let presenter = Presenter::new(&default_theme, default_highlighter, commands, parser, resources, themes, mode);
        presenter.present(&path)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
