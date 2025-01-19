use crate::{
    commands::{SpeakerNotesCommand, listener::CommandListener},
    custom::{Config, ImageProtocol, ValidateOverflows},
    demo::ThemesDemo,
    execute::SnippetExecutor,
    export::Exporter,
    markdown::parse::MarkdownParser,
    presenter::{PresentMode, Presenter, PresenterOptions},
    processing::builder::{PresentationBuilderOptions, Themes},
    render::highlighting::HighlightThemeSet,
    resource::Resources,
    terminal::{
        GraphicsMode,
        image::printer::{ImagePrinter, ImageRegistry},
    },
    theme::{PresentationTheme, PresentationThemeSet},
    third_party::{ThirdPartyConfigs, ThirdPartyRender},
};
use clap::{CommandFactory, Parser, ValueEnum, error::ErrorKind};
use comrak::Arena;
use directories::ProjectDirs;
use iceoryx2::{
    node::NodeBuilder,
    service::{
        builder::publish_subscribe::{Builder, PublishSubscribeCreateError, PublishSubscribeOpenError},
        ipc::Service,
    },
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::{
    env::{self, current_dir},
    io,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

mod ansi;
mod commands;
mod custom;
mod demo;
mod diff;
mod execute;
mod export;
mod markdown;
mod presentation;
mod presenter;
mod processing;
mod render;
mod resource;
mod style;
mod terminal;
mod theme;
mod third_party;
mod tools;

const DEFAULT_THEME: &str = "dark";

#[derive(Clone, Copy, Debug, Deserialize, ValueEnum, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SpeakerNotesMode {
    Publisher,
    Receiver,
}

#[derive(thiserror::Error, Debug)]
enum IpcServiceError {
    #[error("no presenterm process in publisher mode running for presentation")]
    ServiceOpenError,
    #[error("existing presenterm process in publisher mode already running for presentation")]
    ServiceCreateError,
    #[error("{0}")]
    Other(String),
}

impl From<PublishSubscribeOpenError> for IpcServiceError {
    fn from(value: PublishSubscribeOpenError) -> Self {
        match value {
            PublishSubscribeOpenError::DoesNotExist => Self::ServiceOpenError,
            _ => Self::Other(value.to_string()),
        }
    }
}

impl From<PublishSubscribeCreateError> for IpcServiceError {
    fn from(value: PublishSubscribeCreateError) -> Self {
        match value {
            PublishSubscribeCreateError::AlreadyExists => Self::ServiceCreateError,
            _ => Self::Other(value.to_string()),
        }
    }
}

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

    /// Generate a JSON schema for the configuration file.
    #[clap(long)]
    generate_config_file_schema: bool,

    /// Run in export mode.
    #[clap(long, hide = true)]
    enable_export_mode: bool,

    /// Use presentation mode.
    #[clap(short, long, default_value_t = false)]
    present: bool,

    /// The theme to use.
    #[clap(short, long)]
    theme: Option<String>,

    /// List all supported themes.
    #[clap(long)]
    list_themes: bool,

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
    #[clap(short, long)]
    config_file: Option<String>,

    #[clap(short, long)]
    speaker_notes_mode: Option<SpeakerNotesMode>,
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
    code_executor: SnippetExecutor,
}

fn load_customizations(
    config_file_path: Option<PathBuf>,
    cwd: &Path,
) -> Result<Customizations, Box<dyn std::error::Error>> {
    let configs_path: PathBuf = match env::var("XDG_CONFIG_HOME") {
        Ok(path) => Path::new(&path).join("presenterm"),
        Err(_) => {
            let Some(project_dirs) = ProjectDirs::from("", "", "presenterm") else {
                return Ok(Default::default());
            };
            project_dirs.config_dir().into()
        }
    };
    let themes = load_themes(&configs_path)?;
    let config_file_path = config_file_path.unwrap_or_else(|| configs_path.join("config.yaml"));
    let config = Config::load(&config_file_path)?;
    let code_executor = SnippetExecutor::new(config.snippet.exec.custom.clone(), cwd.to_path_buf())?;
    Ok(Customizations { config, themes, code_executor })
}

fn load_themes(config_path: &Path) -> Result<Themes, Box<dyn std::error::Error>> {
    let themes_path = config_path.join("themes");

    let mut highlight_themes = HighlightThemeSet::default();
    highlight_themes.register_from_directory(themes_path.join("highlighting"))?;

    let mut presentation_themes = PresentationThemeSet::default();
    presentation_themes.register_from_directory(&themes_path)?;

    let themes = Themes { presentation: presentation_themes, highlight: highlight_themes };
    Ok(themes)
}

fn display_acknowledgements() {
    let acknowledgements = include_bytes!("../bat/acknowledgements.txt");
    println!("{}", String::from_utf8_lossy(acknowledgements));
}

fn make_builder_options(
    config: &Config,
    mode: &PresentMode,
    force_default_theme: bool,
    speaker_notes_mode: Option<SpeakerNotesMode>,
) -> PresentationBuilderOptions {
    PresentationBuilderOptions {
        allow_mutations: !matches!(mode, PresentMode::Export),
        implicit_slide_ends: config.options.implicit_slide_ends.unwrap_or_default(),
        command_prefix: config.options.command_prefix.clone().unwrap_or_default(),
        image_attribute_prefix: config.options.image_attributes_prefix.clone().unwrap_or_else(|| "image:".to_string()),
        incremental_lists: config.options.incremental_lists.unwrap_or_default(),
        force_default_theme,
        end_slide_shorthand: config.options.end_slide_shorthand.unwrap_or_default(),
        print_modal_background: false,
        strict_front_matter_parsing: config.options.strict_front_matter_parsing.unwrap_or(true),
        enable_snippet_execution: config.snippet.exec.enable,
        enable_snippet_execution_replace: config.snippet.exec_replace.enable,
        render_speaker_notes_only: speaker_notes_mode.is_some_and(|mode| matches!(mode, SpeakerNotesMode::Receiver)),
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
    if cli.enable_export_mode || cli.export_pdf || cli.generate_pdf_metadata {
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

fn overflow_validation(mode: &PresentMode, config: &ValidateOverflows) -> bool {
    match (config, mode) {
        (ValidateOverflows::Always, _) => true,
        (ValidateOverflows::Never, _) => false,
        (ValidateOverflows::WhenPresenting, PresentMode::Presentation) => true,
        (ValidateOverflows::WhenDeveloping, PresentMode::Development) => true,
        _ => false,
    }
}

fn create_speaker_notes_service_builder(
    presentation_path: &Path,
) -> Result<Builder<SpeakerNotesCommand, (), Service>, Box<dyn std::error::Error>> {
    let file_name = presentation_path
        .file_name()
        .ok_or(Cli::command().error(ErrorKind::InvalidValue, "failed to resolve presentation file name"))?
        .to_string_lossy();
    let service_name = format!("presenterm/{file_name}").as_str().try_into()?;
    let service = NodeBuilder::new()
        .create::<Service>()?
        .service_builder(&service_name)
        .publish_subscribe::<SpeakerNotesCommand>()
        .max_publishers(1);
    Ok(service)
}

fn run(mut cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if cli.generate_config_file_schema {
        let schema = schemars::schema_for!(Config);
        serde_json::to_writer_pretty(io::stdout(), &schema).map_err(|e| format!("failed to write schema: {e}"))?;
        return Ok(());
    } else if cli.acknowledgements {
        display_acknowledgements();
        return Ok(());
    } else if cli.list_themes {
        let Customizations { config, themes, .. } =
            load_customizations(cli.config_file.clone().map(PathBuf::from), &current_dir()?)?;
        let bindings = config.bindings.try_into()?;
        let demo = ThemesDemo::new(themes, bindings, io::stdout())?;
        demo.run()?;
        return Ok(());
    }

    let Some(path) = cli.path.take() else {
        Cli::command().error(ErrorKind::MissingRequiredArgument, "no path specified").exit();
    };
    let mut resources_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();
    if resources_path == Path::new("") {
        resources_path = "./".into();
    }
    let resources_path = resources_path.canonicalize().unwrap_or(resources_path);

    let Customizations { config, themes, code_executor } =
        load_customizations(cli.config_file.clone().map(PathBuf::from), &resources_path)?;

    let default_theme = load_default_theme(&config, &themes, &cli);
    let force_default_theme = cli.theme.is_some();
    let mode = match (cli.present, cli.enable_export_mode) {
        (true, _) => PresentMode::Presentation,
        (false, true) => PresentMode::Export,
        (false, false) => PresentMode::Development,
    };
    let arena = Arena::new();
    let parser = MarkdownParser::new(&arena);

    let validate_overflows = overflow_validation(&mode, &config.defaults.validate_overflows) || cli.validate_overflows;
    let mut options = make_builder_options(&config, &mode, force_default_theme, cli.speaker_notes_mode);
    if cli.enable_snippet_execution {
        options.enable_snippet_execution = true;
    }
    if cli.enable_snippet_execution_replace {
        options.enable_snippet_execution_replace = true;
    }
    let graphics_mode = select_graphics_mode(&cli, &config);
    let printer = Arc::new(ImagePrinter::new(graphics_mode.clone())?);
    let registry = ImageRegistry(printer.clone());
    let resources = Resources::new(resources_path.clone(), registry.clone());
    let third_party_config = ThirdPartyConfigs {
        typst_ppi: config.typst.ppi.to_string(),
        mermaid_scale: config.mermaid.scale.to_string(),
        threads: config.snippet.render.threads,
    };
    let third_party = ThirdPartyRender::new(third_party_config, registry, &resources_path);
    let code_executor = Rc::new(code_executor);
    if cli.export_pdf || cli.generate_pdf_metadata {
        let mut exporter =
            Exporter::new(parser, &default_theme, resources, third_party, code_executor, themes, options);
        let mut args = Vec::new();
        if let Some(theme) = cli.theme.as_ref() {
            args.extend(["--theme", theme]);
        }
        if let Some(path) = cli.config_file.as_ref() {
            args.extend(["--config-file", path]);
        }
        if cli.enable_snippet_execution {
            args.push("-x");
        }
        if cli.enable_snippet_execution_replace {
            args.push("-X");
        }
        if cli.export_pdf {
            exporter.export_pdf(&path, &args)?;
        } else {
            let meta = exporter.generate_metadata(&path)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    } else {
        let speaker_notes_event_receiver = if let Some(SpeakerNotesMode::Receiver) = cli.speaker_notes_mode {
            let receiver = create_speaker_notes_service_builder(&path)?
                .open()
                .map_err(|err| Cli::command().error(ErrorKind::InvalidValue, IpcServiceError::from(err)))?
                .subscriber_builder()
                .create()?;
            Some(receiver)
        } else {
            None
        };
        let command_listener = CommandListener::new(config.bindings.clone(), speaker_notes_event_receiver)?;
        options.print_modal_background = matches!(graphics_mode, GraphicsMode::Kitty { .. });

        let speaker_notes_event_publisher = if let Some(SpeakerNotesMode::Publisher) = cli.speaker_notes_mode {
            let publisher = create_speaker_notes_service_builder(&path)?
                .create()
                .map_err(|err| Cli::command().error(ErrorKind::InvalidValue, IpcServiceError::from(err)))?
                .publisher_builder()
                .create()?;
            Some(publisher)
        } else {
            None
        };
        let options = PresenterOptions {
            builder_options: options,
            mode,
            font_size_fallback: config.defaults.terminal_font_size,
            bindings: config.bindings,
            validate_overflows,
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
            speaker_notes_event_publisher,
        );
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
