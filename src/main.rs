use clap::{error::ErrorKind, CommandFactory, Parser};
use comrak::Arena;
use presenterm::{
    CommandSource, Config, Exporter, GraphicsMode, HighlightThemeSet, ImagePrinter, ImageProtocol, ImageRegistry,
    LoadThemeError, MarkdownParser, PresentMode, PresentationBuilderOptions, PresentationTheme, PresentationThemeSet,
    Presenter, PresenterOptions, Resources, Themes, TypstRender,
};
use std::{
    env,
    path::{Path, PathBuf},
    rc::Rc,
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

    /// Display acknowledgements.
    #[clap(long, group = "target")]
    acknowledgements: bool,

    /// The image protocol to use.
    #[clap(long)]
    image_protocol: Option<ImageProtocol>,

    /// The path to the configuration file.
    #[clap(short, long)]
    config_file: Option<String>,
}

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

fn load_customizations(config_file_path: Option<PathBuf>) -> Result<(Config, Themes), Box<dyn std::error::Error>> {
    let Ok(home_path) = env::var("HOME") else {
        return Ok(Default::default());
    };
    let home_path = PathBuf::from(home_path);
    let configs_path = home_path.join(".config/presenterm");
    let themes = load_themes(&configs_path)?;
    let config_file_path = config_file_path.unwrap_or_else(|| configs_path.join("config.yaml"));
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
        print_modal_background: false,
    }
}

fn load_default_theme(config: &Config, themes: &Themes, cli: &Cli) -> PresentationTheme {
    let default_theme_name =
        cli.theme.as_ref().or(config.defaults.theme.as_ref()).map(|s| s.as_str()).unwrap_or(DEFAULT_THEME);
    let Some(default_theme) = themes.presentation.load_by_name(default_theme_name) else {
        let valid_themes = themes.presentation.theme_names().join(", ");
        let error_message = format!("invalid theme name, valid themes are: {valid_themes}");
        Cli::command().error(ErrorKind::InvalidValue, error_message).exit();
    };
    default_theme
}

fn select_graphics_mode(cli: &Cli, config: &Config) -> GraphicsMode {
    if cli.export || cli.export_pdf || cli.generate_pdf_metadata {
        GraphicsMode::AsciiBlocks
    } else {
        let protocol = cli.image_protocol.as_ref().unwrap_or(&config.defaults.image_protocol);
        match GraphicsMode::try_from(protocol) {
            Ok(mode) => mode,
            Err(_) => {
                Cli::command().error(ErrorKind::InvalidValue, "sixel support was not enabled during compilation").exit()
            }
        }
    }
}

fn run(mut cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let (config, themes) = load_customizations(cli.config_file.clone().map(PathBuf::from))?;

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

    let path = cli.path.take().unwrap_or_else(|| {
        Cli::command().error(ErrorKind::MissingRequiredArgument, "no path specified").exit();
    });
    let resources_path = path.parent().unwrap_or(Path::new("/"));
    let mut options = make_builder_options(&config, &mode, force_default_theme);
    let graphics_mode = select_graphics_mode(&cli, &config);
    let printer = Rc::new(ImagePrinter::new(graphics_mode.clone())?);
    let registry = ImageRegistry(printer.clone());
    let resources = Resources::new(resources_path, registry.clone());
    let typst = TypstRender::new(config.typst.ppi, registry);
    if cli.export_pdf || cli.generate_pdf_metadata {
        let mut exporter = Exporter::new(parser, &default_theme, resources, typst, themes, options);
        let mut args = Vec::new();
        if let Some(theme) = cli.theme.as_ref() {
            args.extend(["--theme", theme]);
        }
        if let Some(path) = cli.config_file.as_ref() {
            args.extend(["--config-file", &path]);
        }
        if cli.export_pdf {
            exporter.export_pdf(&path, &args)?;
        } else {
            let meta = exporter.generate_metadata(&path)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    } else {
        let commands = CommandSource::new(&path, config.bindings.clone())?;
        options.print_modal_background = matches!(graphics_mode, GraphicsMode::Kitty { .. });

        let options = PresenterOptions {
            builder_options: options,
            mode,
            font_size_fallback: config.defaults.terminal_font_size,
            bindings: config.bindings,
        };
        let presenter = Presenter::new(&default_theme, commands, parser, resources, typst, themes, printer, options);
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
