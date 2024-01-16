use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use comrak::Arena;
use presenterm::{
    run_demo, CommandSource, Config, Exporter, GraphicsMode, HighlightThemeSet, LoadThemeError, MarkdownParser,
    MediaRender, PresentMode, PresentationBuilderOptions, PresentationTheme, PresentationThemeSet, Presenter,
    PresenterOptions, Resources, Themes, TypstRender,
};
use std::{
    env,
    path::{Path, PathBuf},
};

const DEFAULT_THEME: &str = "dark";

/// Run slideshows from your terminal.
#[derive(Parser)]
#[command()]
#[command(author, version, about = create_splash(), arg_required_else_help = true)]
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
    #[clap(short, long)]
    theme: Option<String>,

    /// Grabs demo from repo and presents it.
    #[clap(long, conflicts_with = "target")]
    demo: bool,

    /// Display acknowledgements.
    #[clap(long, group = "target")]
    acknowledgements: bool,

    /// The preferred image protocol.
    #[clap(long)]
    image_protocol: Option<ImageProtocol>,
}

#[derive(Clone, Debug, ValueEnum)]
enum ImageProtocol {
    Iterm2,
    Kitty,
    Sixel,
    AsciiBlocks,
}

impl TryFrom<ImageProtocol> for GraphicsMode {
    type Error = SixelUnsupported;

    fn try_from(protocol: ImageProtocol) -> Result<Self, Self::Error> {
        let mode = match protocol {
            ImageProtocol::Iterm2 => GraphicsMode::Iterm2,
            ImageProtocol::Kitty => GraphicsMode::Kitty,
            ImageProtocol::AsciiBlocks => GraphicsMode::AsciiBlocks,
            #[cfg(feature = "sixel")]
            ImageProtocol::Sixel => GraphicsMode::Sixel,
            #[cfg(not(feature = "sixel"))]
            ImageProtocol::Sixel => return Err(SixelUnsupported),
        };
        Ok(mode)
    }
}

struct SixelUnsupported;

fn create_splash() -> String {
    let crate_version = env!("CARGO_PKG_VERSION");

    format!(
        r#"
  ┌─┐┬─┐┌─┐┌─┐┌─┐┌┐┌┌┬┐┌─┐┬─┐┌┬┐
  ├─┘├┬┘├┤ └─┐├┤ │││ │ ├┤ ├┬┘│││
  ┴  ┴└─└─┘└─┘└─┘┘└┘ ┴ └─┘┴└─┴ ┴ v{crate_version}
    A terminal slideshow tool 
                    @mfontanini/presenterm
"#,
    )
}

fn load_customizations() -> Result<(Config, Themes), Box<dyn std::error::Error>> {
    let Ok(home_path) = env::var("HOME") else {
        return Ok(Default::default());
    };
    let home_path = PathBuf::from(home_path);
    let configs_path = home_path.join(".config/presenterm");
    let themes = load_themes(&configs_path)?;
    let config_file_path = configs_path.join("config.yaml");
    let config = Config::load(&config_file_path)?;
    Ok((config, themes))
}

fn load_themes(config_path: &Path) -> Result<Themes, Box<dyn std::error::Error>> {
    let themes_path = config_path.join("themes");

    let mut highlight_themes = HighlightThemeSet::default();
    highlight_themes.register_from_directory(&themes_path.join("highlighting"))?;

    let mut presentation_themes = PresentationThemeSet::default();
    let register_result = presentation_themes.register_from_directory(&themes_path);
    if let Err(e @ (LoadThemeError::Duplicate(_) | LoadThemeError::Corrupted(..))) = register_result {
        return Err(e.into());
    }

    let themes = Themes { presentation: presentation_themes, highlight: highlight_themes };
    Ok(themes)
}

fn display_acknowledgements() {
    let acknowledgements = include_bytes!("../bat/acknowledgements.txt");
    println!("{}", String::from_utf8_lossy(acknowledgements));
}

fn make_builder_options(config: &Config, mode: &PresentMode, force_default_theme: bool) -> PresentationBuilderOptions {
    PresentationBuilderOptions {
        allow_mutations: !matches!(mode, PresentMode::Export),
        implicit_slide_ends: config.options.implicit_slide_ends.unwrap_or_default(),
        command_prefix: config.options.command_prefix.clone().unwrap_or_default(),
        incremental_lists: config.options.incremental_lists.unwrap_or_default(),
        force_default_theme,
        end_slide_shorthand: config.options.end_slide_shorthand.unwrap_or_default(),
    }
}

fn load_default_theme(config: &Config, themes: &Themes, cli: &Cli) -> PresentationTheme {
    let default_theme_name =
        cli.theme.as_ref().or(config.defaults.theme.as_ref()).map(|s| s.as_str()).unwrap_or(DEFAULT_THEME);
    let Some(default_theme) = themes.presentation.load_by_name(default_theme_name) else {
        let mut cmd = Cli::command();
        let valid_themes = themes.presentation.theme_names().join(", ");
        let error_message = format!("invalid theme name, valid themes are: {valid_themes}");
        cmd.error(ErrorKind::InvalidValue, error_message).exit();
    };
    default_theme
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let (config, themes) = load_customizations()?;

    let default_theme = load_default_theme(&config, &themes, &cli);
    let force_default_theme = cli.theme.is_some();
    let mode = match (cli.present, cli.export) {
        (true, _) => PresentMode::Presentation,
        (false, true) => PresentMode::Export,
        (false, false) => PresentMode::Development,
    };
    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    if cli.acknowledgements {
        display_acknowledgements();
        return Ok(());
    }
    // Pre-load this so we don't flicker on the first displayed image.
    MediaRender::detect_terminal_protocol();

    let path = if cli.demo {
        let temp_dir = Path::new("/tmp/presenterm-demo");
        run_demo(temp_dir)?;
        temp_dir.join("demo.md").to_path_buf()
    } else {
        cli.path.unwrap_or_else(|| {
            eprintln!("Error: No path specified.");
            std::process::exit(1);
        })
    };

    let resources_path = path.parent().unwrap_or(Path::new("/"));
    let resources = Resources::new(resources_path);
    let typst = TypstRender::new(config.typst.ppi);
    let options = make_builder_options(&config, &mode, force_default_theme);
    if cli.export_pdf || cli.generate_pdf_metadata {
        let mut exporter = Exporter::new(parser, &default_theme, resources, typst, themes, options);
        let mut args = Vec::new();
        if let Some(theme) = cli.theme.as_ref() {
            args.extend(["--theme", theme]);
        }
        if cli.export_pdf {
            exporter.export_pdf(&path, &args)?;
        } else {
            let meta = exporter.generate_metadata(&path)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    } else {
        let commands = CommandSource::new(&path, config.bindings)?;
        let graphics_mode = match cli.image_protocol.map(GraphicsMode::try_from) {
            Some(Ok(mode)) => mode,
            Some(Err(_)) => {
                let mut cmd = Cli::command();
                cmd.error(ErrorKind::InvalidValue, "sixel support was not enabled during compilation").exit();
            }
            None => GraphicsMode::default(),
        };
        let options = PresenterOptions {
            builder_options: options,
            mode,
            graphics_mode,
            font_size_fallback: config.defaults.terminal_font_size,
        };
        let presenter = Presenter::new(&default_theme, commands, parser, resources, typst, themes, options);
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
