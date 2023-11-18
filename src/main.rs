use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    CodeHighlighter, CommandSource, Exporter, MarkdownParser, PresentMode, PresentationTheme, Presenter, Resources,
};
use std::path::{Path, PathBuf};

/// Run slideshows from your terminal.
#[derive(Parser)]
#[command()]
#[command(author, version, about = create_splash(), long_about = create_splash(), arg_required_else_help = true)]
struct Cli {
    /// The path to the markdown file that contains the presentation.
    path: PathBuf,

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

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let Some(default_theme) = PresentationTheme::from_name(&cli.theme) else {
        let mut cmd = Cli::command();
        let valid_themes = PresentationTheme::theme_names().collect::<Vec<_>>().join(", ");
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
    let default_highlighter = CodeHighlighter::new("base16-ocean.dark")?;
    let resources_path = cli.path.parent().unwrap_or(Path::new("/"));
    let resources = Resources::new(resources_path);
    if cli.export_pdf || cli.generate_pdf_metadata {
        let mut exporter = Exporter::new(parser, &default_theme, default_highlighter, resources);
        if cli.export_pdf {
            exporter.export_pdf(&cli.path)?;
        } else {
            let meta = exporter.generate_metadata(&cli.path)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    } else {
        let commands = CommandSource::new(&cli.path);
        let presenter = Presenter::new(&default_theme, default_highlighter, commands, parser, resources, mode);
        presenter.present(&cli.path)?;
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
