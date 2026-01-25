use crate::{
    code::{execute::SnippetExecutor, highlighting::HighlightThemeSet},
    commands::listener::CommandListener,
    config::{Config, ImageProtocol, ValidateOverflows},
    demo::ThemesDemo,
    export::exporter::Exporter,
    markdown::parse::MarkdownParser,
    presentation::builder::{CommentCommand, PresentationBuilderOptions, Themes},
    presenter::{PresentMode, Presenter, PresenterOptions},
    resource::Resources,
    terminal::{
        GraphicsMode,
        image::printer::{ImagePrinter, ImageRegistry},
    },
    theme::{raw::PresentationTheme, registry::PresentationThemeRegistry},
    third_party::{ThirdPartyConfigs, ThirdPartyRender},
};
use anyhow::anyhow;
use clap::{CommandFactory, Parser, error::ErrorKind};
use commands::speaker_notes::{SpeakerNotesEventListener, SpeakerNotesEventPublisher};
use comrak::Arena;
use config::ConfigLoadError;
use crossterm::{
    execute,
    style::{PrintStyledContent, Stylize},
};
use directories::ProjectDirs;
use export::exporter::OutputDirectory;
use render::{engine::MaxSize, properties::WindowSize};
use std::{
    env::{self, current_dir},
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use terminal::emulator::TerminalEmulator;
use theme::ThemeOptions;

mod code;
mod commands;
mod config;
mod demo;
mod export;
mod markdown;
mod presentation;
mod presenter;
mod render;
mod resource;
mod terminal;
mod theme;
mod third_party;
mod tools;
mod transitions;
mod ui;
mod utils;

const DEFAULT_THEME: &str = "dark";
const DEFAULT_THEME_DYNAMIC_DETECTION_TIMEOUT: u64 = 100;
const DEFAULT_EXPORT_PIXELS_PER_COLUMN: u16 = 20;
const DEFAULT_EXPORT_PIXELS_PER_ROW: u16 = DEFAULT_EXPORT_PIXELS_PER_COLUMN * 2;

/// Run slideshows from your terminal.
#[derive(Parser)]
#[command()]
#[command(author, version, about = create_splash(), arg_required_else_help = true)]
struct Cli {
    /// The path to the markdown file that contains the presentation.
    #[clap(group = "target")]
    path: Option<PathBuf>,

    /// Export the presentation as a PDF rather than displaying it.
    #[clap(short, long, group = "export")]
    export_pdf: bool,

    /// Export the presentation as a HTML rather than displaying it.
    #[clap(short = 'E', long, group = "export")]
    export_html: bool,

    /// The path in which to store temporary files used when exporting.
    #[clap(long, requires = "export")]
    export_temporary_path: Option<PathBuf>,

    /// The output path for the exported PDF.
    #[clap(short = 'o', long = "output", requires = "export")]
    export_output: Option<PathBuf>,

    /// Generate a JSON schema for the configuration file.
    #[clap(long)]
    #[cfg(feature = "json-schema")]
    generate_config_file_schema: bool,

    /// Use presentation mode.
    #[clap(short, long, default_value_t = false)]
    present: bool,

    /// The theme to use.
    #[clap(short, long)]
    theme: Option<String>,

    /// List all supported themes.
    #[clap(long, group = "target")]
    list_themes: bool,

    /// Print the theme in use.
    #[clap(long, group = "target")]
    current_theme: bool,

    /// Display acknowledgements.
    #[clap(long, group = "target")]
    acknowledgements: bool,

    /// The image protocol to use.
    #[clap(long)]
    image_protocol: Option<ImageProtocol>,

    /// Validate that the presentation does not overflow the terminal screen.
    #[clap(long)]
    validate_overflows: bool,

    /// Enable code snippet execution.
    #[clap(short = 'x', long)]
    enable_snippet_execution: bool,

    /// Enable code snippet auto execution via `+exec_replace` blocks.
    #[clap(short = 'X', long)]
    enable_snippet_execution_replace: bool,

    /// The path to the configuration file.
    #[clap(short, long, env = "PRESENTERM_CONFIG_FILE")]
    config_file: Option<String>,

    /// Whether to publish speaker notes to local listeners.
    #[clap(short = 'P', long, group = "speaker-notes")]
    publish_speaker_notes: bool,

    /// Whether to listen for speaker notes.
    #[clap(short, long, group = "speaker-notes")]
    listen_speaker_notes: bool,

    /// Whether to validate snippets.
    #[clap(long)]
    validate_snippets: bool,

    /// List all available comment commands.
    #[clap(long, group = "target")]
    list_comment_commands: bool,
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

#[derive(Default)]
struct Customizations {
    config: Config,
    themes: Themes,
    themes_path: Option<PathBuf>,
    code_executor: SnippetExecutor,
}

impl Customizations {
    fn load(config_file_path: Option<PathBuf>, cwd: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let configs_path: PathBuf = match env::var("XDG_CONFIG_HOME") {
            Ok(path) => Path::new(&path).join("presenterm"),
            Err(_) => {
                let Some(project_dirs) = ProjectDirs::from("", "", "presenterm") else {
                    return Ok(Default::default());
                };
                project_dirs.config_dir().into()
            }
        };
        let themes_path = configs_path.join("themes");
        let themes = Self::load_themes(&themes_path)?;
        let require_config_file = config_file_path.is_some();
        let config_file_path = config_file_path.unwrap_or_else(|| configs_path.join("config.yaml"));
        let config = match Config::load(&config_file_path) {
            Ok(config) => config,
            Err(ConfigLoadError::NotFound) if !require_config_file => Default::default(),
            Err(e) => return Err(e.into()),
        };
        let code_executor = SnippetExecutor::new(config.snippet.exec.custom.clone(), cwd.to_path_buf())?;
        Ok(Customizations { config, themes, themes_path: Some(themes_path), code_executor })
    }

    fn load_themes(themes_path: &Path) -> Result<Themes, Box<dyn std::error::Error>> {
        let mut highlight_themes = HighlightThemeSet::default();
        highlight_themes.register_from_directory(themes_path.join("highlighting"))?;

        let mut presentation_themes = PresentationThemeRegistry::default();
        presentation_themes.register_from_directory(themes_path)?;

        let themes = Themes { presentation: presentation_themes, highlight: highlight_themes };
        Ok(themes)
    }
}

struct CoreComponents {
    third_party: ThirdPartyRender,
    code_executor: Arc<SnippetExecutor>,
    resources: Resources,
    printer: Arc<ImagePrinter>,
    builder_options: PresentationBuilderOptions,
    themes: Themes,
    default_theme: PresentationTheme,
    config: Config,
    present_mode: PresentMode,
    graphics_mode: GraphicsMode,
}

impl CoreComponents {
    fn new(cli: &Cli, path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut resources_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();
        if resources_path == Path::new("") {
            resources_path = "./".into();
        }
        let resources_path = resources_path.canonicalize().unwrap_or(resources_path);

        let Customizations { config, themes, code_executor, themes_path } =
            Customizations::load(cli.config_file.clone().map(PathBuf::from), &resources_path)?;

        let default_theme = Self::load_default_theme(&config, &themes, cli);
        let force_default_theme = cli.theme.is_some();
        let present_mode = match (cli.present, cli.export_pdf) {
            (true, _) | (_, true) => PresentMode::Presentation,
            (false, false) => PresentMode::Development,
        };

        let mut builder_options = Self::make_builder_options(&config, force_default_theme, cli.listen_speaker_notes);
        if cli.enable_snippet_execution {
            builder_options.enable_snippet_execution = true;
        }
        if cli.enable_snippet_execution_replace {
            builder_options.enable_snippet_execution_replace = true;
        }
        let graphics_mode = Self::select_graphics_mode(cli, &config);
        let printer = Arc::new(ImagePrinter::new(graphics_mode.clone())?);
        let registry = ImageRegistry::new(printer.clone());
        let resources = Resources::new(
            resources_path.clone(),
            themes_path.unwrap_or_else(|| resources_path.clone()),
            registry.clone(),
        );
        let third_party_config = ThirdPartyConfigs {
            typst_ppi: config.typst.ppi.to_string(),
            mermaid_scale: config.mermaid.scale.to_string(),
            mermaid_pupeteer_file: config.mermaid.pupeteer_config_path.clone(),
            d2_scale: config.d2.scale.map(|s| s.to_string()).unwrap_or_else(|| "-1".to_string()),
            threads: config.snippet.render.threads,
        };
        let third_party = ThirdPartyRender::new(third_party_config, registry, &resources_path);
        let code_executor = Arc::new(code_executor);
        Ok(Self {
            third_party,
            code_executor,
            resources,
            printer,
            builder_options,
            themes,
            default_theme,
            config,
            present_mode,
            graphics_mode,
        })
    }

    fn make_builder_options(
        config: &Config,
        force_default_theme: bool,
        render_speaker_notes_only: bool,
    ) -> PresentationBuilderOptions {
        let options = &config.options;
        PresentationBuilderOptions {
            allow_mutations: true,
            implicit_slide_ends: options.implicit_slide_ends.unwrap_or_default(),
            command_prefix: options.command_prefix.clone().unwrap_or_default(),
            image_attribute_prefix: options.image_attributes_prefix.clone().unwrap_or_else(|| "image:".to_string()),
            incremental_lists: options.incremental_lists.unwrap_or_default(),
            force_default_theme,
            end_slide_shorthand: options.end_slide_shorthand.unwrap_or_default(),
            print_modal_background: false,
            strict_front_matter_parsing: options.strict_front_matter_parsing.unwrap_or(true),
            enable_snippet_execution: config.snippet.exec.enable,
            enable_snippet_execution_replace: config.snippet.exec_replace.enable,
            render_speaker_notes_only,
            auto_render_languages: options.auto_render_languages.clone(),
            theme_options: ThemeOptions { font_size_supported: TerminalEmulator::capabilities().font_size },
            pause_before_incremental_lists: config.defaults.incremental_lists.pause_before.unwrap_or(true),
            pause_after_incremental_lists: config.defaults.incremental_lists.pause_after.unwrap_or(true),
            pause_create_new_slide: false,
            list_item_newlines: options.list_item_newlines.map(Into::into).unwrap_or(1),
            validate_snippets: config.snippet.validate,
            layout_grid: false,
            h1_slide_titles: options.h1_slide_titles.unwrap_or_default(),
        }
    }

    fn select_graphics_mode(cli: &Cli, config: &Config) -> GraphicsMode {
        if cli.export_pdf | cli.export_html {
            GraphicsMode::Raw
        } else {
            cli.image_protocol.as_ref().unwrap_or(&config.defaults.image_protocol).into()
        }
    }

    fn theme_name(config: &Config, cli: &Cli) -> String {
        if let Some(name) = cli.theme.as_ref() {
            name.clone()
        } else {
            match &config.defaults.theme {
                config::ThemeConfig::None => DEFAULT_THEME.into(),
                config::ThemeConfig::Some(theme_name) => theme_name.clone(),
                config::ThemeConfig::Dynamic { dark, light, timeout } => {
                    let default_timeout = timeout.unwrap_or(DEFAULT_THEME_DYNAMIC_DETECTION_TIMEOUT);
                    let timeout_duration = Duration::from_millis(default_timeout);
                    if let Ok(theme) = termbg::theme(timeout_duration) {
                        if theme == termbg::Theme::Dark { dark.clone() } else { light.clone() }
                    } else {
                        Cli::command()
                            .error(
                                ErrorKind::Io,
                                "terminal theme detection failed, unsupported terminal or timeout exceeded",
                            )
                            .exit();
                    }
                }
            }
        }
    }

    fn load_default_theme(config: &Config, themes: &Themes, cli: &Cli) -> PresentationTheme {
        let default_theme_name = Self::theme_name(config, cli);
        let Some(default_theme) = themes.presentation.load_by_name(default_theme_name.as_str()) else {
            let valid_themes = themes.presentation.theme_names().join(", ");
            let error_message = format!("invalid theme name, valid themes are: {valid_themes}");
            Cli::command().error(ErrorKind::InvalidValue, error_message).exit();
        };
        default_theme
    }
}

struct SpeakerNotesComponents {
    events_listener: Option<SpeakerNotesEventListener>,
    events_publisher: Option<SpeakerNotesEventPublisher>,
}

impl SpeakerNotesComponents {
    fn new(cli: &Cli, config: &Config, path: &Path) -> anyhow::Result<Self> {
        let full_presentation_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let publish_speaker_notes =
            cli.publish_speaker_notes || (config.speaker_notes.always_publish && !cli.listen_speaker_notes);
        let events_publisher = publish_speaker_notes
            .then(|| {
                SpeakerNotesEventPublisher::new(config.speaker_notes.publish_address, full_presentation_path.clone())
            })
            .transpose()
            .map_err(|e| anyhow!("failed to create speaker notes publisher: {e}"))?;
        let events_listener = cli
            .listen_speaker_notes
            .then(|| SpeakerNotesEventListener::new(config.speaker_notes.listen_address, full_presentation_path))
            .transpose()
            .map_err(|e| anyhow!("failed to create speaker notes listener: {e}"))?;
        Ok(Self { events_listener, events_publisher })
    }
}

fn overflow_validation_enabled(mode: &PresentMode, config: &ValidateOverflows) -> bool {
    match (config, mode) {
        (ValidateOverflows::Always, _) => true,
        (ValidateOverflows::Never, _) => false,
        (ValidateOverflows::WhenPresenting, PresentMode::Presentation) => true,
        (ValidateOverflows::WhenDeveloping, PresentMode::Development) => true,
        _ => false,
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "json-schema")]
    if cli.generate_config_file_schema {
        let schema = schemars::schema_for!(Config);
        serde_json::to_writer_pretty(io::stdout(), &schema).map_err(|e| format!("failed to write schema: {e}"))?;
        return Ok(());
    }
    if cli.acknowledgements {
        let acknowledgements = include_bytes!("../bat/acknowledgements.txt");
        println!("{}", String::from_utf8_lossy(acknowledgements));
        return Ok(());
    } else if cli.list_themes {
        // Load this ahead of time so we don't do it when we're already in raw mode.
        TerminalEmulator::capabilities();
        let Customizations { config, themes, .. } =
            Customizations::load(cli.config_file.clone().map(PathBuf::from), &current_dir()?)?;
        let bindings = config.bindings.try_into()?;
        let demo = ThemesDemo::new(themes, bindings)?;
        demo.run()?;
        return Ok(());
    } else if cli.current_theme {
        let Customizations { config, .. } =
            Customizations::load(cli.config_file.clone().map(PathBuf::from), &current_dir()?)?;
        let theme_name = CoreComponents::theme_name(&config, &cli);
        println!("{theme_name}");
        return Ok(());
    } else if cli.list_comment_commands {
        let samples = CommentCommand::generate_samples();
        for sample in samples {
            println!("{}", sample);
        }
        return Ok(());
    }
    // Disable this so we don't mess things up when generating PDFs
    if cli.export_pdf {
        TerminalEmulator::disable_capability_detection();
    }

    let Some(path) = cli.path.clone() else {
        Cli::command().error(ErrorKind::MissingRequiredArgument, "no path specified").exit();
    };
    let CoreComponents {
        third_party,
        code_executor,
        resources,
        printer,
        mut builder_options,
        themes,
        default_theme,
        config,
        present_mode,
        graphics_mode,
    } = CoreComponents::new(&cli, &path)?;
    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);
    let validate_overflows =
        overflow_validation_enabled(&present_mode, &config.defaults.validate_overflows) || cli.validate_overflows;
    if cli.validate_snippets {
        builder_options.validate_snippets = cli.validate_snippets;
    }
    if cli.export_pdf || cli.export_html {
        let dimensions = match config.export.dimensions {
            Some(dimensions) => WindowSize {
                rows: dimensions.rows,
                columns: dimensions.columns,
                height: dimensions.rows * DEFAULT_EXPORT_PIXELS_PER_ROW,
                width: dimensions.columns * DEFAULT_EXPORT_PIXELS_PER_COLUMN,
            },
            None => WindowSize::current(config.defaults.terminal_font_size)?,
        };
        let exporter = Exporter::new(
            parser,
            &default_theme,
            resources,
            third_party,
            code_executor,
            printer,
            themes,
            builder_options,
            dimensions,
            config.export.pauses,
            config.export.snippets,
        );
        let output_directory = match cli.export_temporary_path {
            Some(path) => OutputDirectory::external(path),
            None => OutputDirectory::temporary(),
        }?;
        if cli.export_pdf {
            exporter.export_pdf(&path, output_directory, cli.export_output.as_deref(), config.export.pdf)?;
        } else {
            exporter.export_html(&path, output_directory, cli.export_output.as_deref())?;
        }
    } else {
        let SpeakerNotesComponents { events_listener, events_publisher } =
            SpeakerNotesComponents::new(&cli, &config, &path)?;
        let command_listener = CommandListener::new(config.bindings.clone(), events_listener)?;

        builder_options.print_modal_background = matches!(graphics_mode, GraphicsMode::Kitty { .. });
        let options = PresenterOptions {
            builder_options,
            mode: present_mode,
            font_size_fallback: config.defaults.terminal_font_size,
            bindings: config.bindings,
            validate_overflows,
            max_size: MaxSize {
                max_columns: config.defaults.max_columns,
                max_columns_alignment: config.defaults.max_columns_alignment,
                max_rows: config.defaults.max_rows,
                max_rows_alignment: config.defaults.max_rows_alignment,
            },
            transition: config.transition,
        };
        let presenter = Presenter::new(
            &default_theme,
            command_listener,
            parser,
            resources,
            third_party,
            code_executor,
            themes,
            printer,
            options,
            events_publisher,
        );
        presenter.present(&path)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        let _ =
            execute!(io::stdout(), PrintStyledContent(format!("{e}\n").stylize().with(crossterm::style::Color::Red)));
        std::process::exit(1);
    }
}
