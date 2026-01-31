use crate::{
    ImageRegistry,
    config::{default_mermaid_scale, default_snippet_render_threads, default_typst_ppi},
    markdown::{
        elements::{Line, Percent, Text},
        text_style::{Color, TextStyle},
    },
    render::{
        operation::{
            AsRenderOperations, ImageRenderProperties, ImageSize, Pollable, PollableState, RenderAsync,
            RenderAsyncStartPolicy, RenderOperation,
        },
        properties::WindowSize,
    },
    terminal::image::{
        Image,
        printer::{ImageSpec, RegisterImageError},
    },
    theme::{Alignment, D2Style, MermaidStyle, PresentationTheme, TypstStyle, raw::RawColor},
    tools::{ExecutionError, ThirdPartyTools},
};
use std::{
    collections::{HashMap, VecDeque},
    fs, io, mem,
    path::Path,
    rc::Rc,
    sync::{Arc, Condvar, Mutex},
    thread,
};

pub struct ThirdPartyConfigs {
    pub typst_ppi: String,
    pub mermaid_scale: String,
    pub mermaid_pupeteer_file: Option<String>,
    pub mermaid_config_file: Option<String>,
    pub d2_scale: String,
    pub threads: usize,
}

pub struct ThirdPartyRender {
    render_pool: RenderPool,
}

impl ThirdPartyRender {
    pub fn new(config: ThirdPartyConfigs, image_registry: ImageRegistry, root_dir: &Path) -> Self {
        // typst complains about empty paths so we give it a "." if we don't have one.
        let root_dir = match root_dir.to_string_lossy().to_string() {
            path if path.is_empty() => ".".into(),
            path => path,
        };
        let render_pool = RenderPool::new(config, root_dir, image_registry);
        Self { render_pool }
    }

    pub(crate) fn render(
        &self,
        request: ThirdPartyRenderRequest,
        theme: &PresentationTheme,
        width: Option<Percent>,
    ) -> Result<RenderOperation, ThirdPartyRenderError> {
        let result = self.render_pool.render(request);
        let operation = Rc::new(RenderThirdParty::new(result, theme.default_style.style, width));
        Ok(RenderOperation::RenderAsync(operation))
    }
}

impl Default for ThirdPartyRender {
    fn default() -> Self {
        let config = ThirdPartyConfigs {
            typst_ppi: default_typst_ppi().to_string(),
            mermaid_scale: default_mermaid_scale().to_string(),
            mermaid_pupeteer_file: None,
            mermaid_config_file: None,
            d2_scale: "-1".to_string(),
            threads: default_snippet_render_threads(),
        };
        Self::new(config, Default::default(), Path::new("."))
    }
}

#[derive(Debug)]
pub(crate) enum ThirdPartyRenderRequest {
    Typst(String, TypstStyle),
    Latex(String, TypstStyle),
    Mermaid(String, MermaidStyle),
    D2(String, D2Style),
}

#[derive(Debug, Default)]
enum RenderResult {
    Success(Image),
    Failure(String),
    #[default]
    Pending,
}

struct RenderPoolState {
    requests: VecDeque<(ThirdPartyRenderRequest, Arc<Mutex<RenderResult>>)>,
    image_registry: ImageRegistry,
    cache: HashMap<ImageSnippet, Image>,
}

struct Shared {
    config: ThirdPartyConfigs,
    root_dir: String,
    signal: Condvar,
}

struct RenderPool {
    state: Arc<Mutex<RenderPoolState>>,
    shared: Arc<Shared>,
}

impl RenderPool {
    fn new(config: ThirdPartyConfigs, root_dir: String, image_registry: ImageRegistry) -> Self {
        let threads = config.threads;
        let shared = Shared { config, root_dir, signal: Default::default() };
        let state = RenderPoolState { requests: Default::default(), image_registry, cache: Default::default() };

        let this = Self { state: Arc::new(Mutex::new(state)), shared: Arc::new(shared) };
        for _ in 0..threads {
            let worker = Worker { state: this.state.clone(), shared: this.shared.clone() };
            thread::spawn(move || worker.run());
        }
        this
    }

    fn render(&self, request: ThirdPartyRenderRequest) -> Arc<Mutex<RenderResult>> {
        let result: Arc<Mutex<RenderResult>> = Default::default();
        let mut state = self.state.lock().expect("lock poisoned");
        state.requests.push_back((request, result.clone()));
        self.shared.signal.notify_one();
        result
    }
}

struct Worker {
    state: Arc<Mutex<RenderPoolState>>,
    shared: Arc<Shared>,
}

impl Worker {
    fn run(self) {
        loop {
            let mut state = self.state.lock().unwrap();
            let (request, result) = loop {
                let Some((request, result)) = state.requests.pop_front() else {
                    state = self.shared.signal.wait(state).unwrap();
                    continue;
                };
                break (request, result);
            };
            drop(state);

            self.render(request, result);
        }
    }

    fn render(&self, request: ThirdPartyRenderRequest, result: Arc<Mutex<RenderResult>>) {
        let output = match request {
            ThirdPartyRenderRequest::Typst(input, style) => self.render_typst(input, &style),
            ThirdPartyRenderRequest::Latex(input, style) => self.render_latex(input, &style),
            ThirdPartyRenderRequest::Mermaid(input, style) => self.render_mermaid(input, &style),
            ThirdPartyRenderRequest::D2(input, style) => self.render_d2(input, &style),
        };
        let mut result = result.lock().unwrap();
        match output {
            Ok(image) => *result = RenderResult::Success(image),
            Err(error) => *result = RenderResult::Failure(error.to_string()),
        };
    }

    pub(crate) fn render_typst(&self, input: String, style: &TypstStyle) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: input.clone(), source: SnippetSource::Typst };
        if let Some(image) = self.state.lock().unwrap().cache.get(&snippet).cloned() {
            return Ok(image);
        }
        self.do_render_typst(snippet, &input, style)
    }

    pub(crate) fn render_latex(&self, input: String, style: &TypstStyle) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: input.clone(), source: SnippetSource::Latex };
        if let Some(image) = self.state.lock().unwrap().cache.get(&snippet).cloned() {
            return Ok(image);
        }
        let output = ThirdPartyTools::pandoc(&["--from", "latex", "--to", "typst"])
            .stdin(input.as_bytes().into())
            .run_and_capture_stdout()?;

        let input = String::from_utf8_lossy(&output);
        self.do_render_typst(snippet, &input, style)
    }

    pub(crate) fn render_mermaid(&self, input: String, style: &MermaidStyle) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: input.clone(), source: SnippetSource::Mermaid };
        if let Some(image) = self.state.lock().unwrap().cache.get(&snippet).cloned() {
            return Ok(image);
        }
        let workdir = tempfile::Builder::default().prefix(".presenterm").tempdir()?;
        let output_path = workdir.path().join("output.png");
        let input_path = workdir.path().join("input.mmd");
        fs::write(&input_path, input)?;

        let input_path = input_path.to_string_lossy();
        let output_path_str = output_path.to_string_lossy();
        let mut args = vec![
            "-i",
            &input_path,
            "-o",
            &output_path_str,
            "-s",
            &self.shared.config.mermaid_scale,
            "-t",
            &style.theme,
            "-b",
            &style.background,
        ];
        if let Some(path) = &self.shared.config.mermaid_pupeteer_file {
            args.extend(&["-p", path]);
        }
        if let Some(path) = &self.shared.config.mermaid_config_file {
            args.extend(&["-c", path]);
        }

        ThirdPartyTools::mermaid(&args).run()?;

        self.load_image(snippet, &output_path)
    }

    pub(crate) fn render_d2(&self, input: String, style: &D2Style) -> Result<Image, ThirdPartyRenderError> {
        let snippet = ImageSnippet { snippet: input.clone(), source: SnippetSource::D2 };
        let workdir = tempfile::Builder::default().prefix(".presenterm").tempdir()?;
        let output_path = workdir.path().join("output.png");
        let input_path = workdir.path().join("input.d2");
        fs::write(&input_path, input)?;
        ThirdPartyTools::d2(&[
            &input_path.to_string_lossy(),
            &output_path.to_string_lossy(),
            "--pad",
            "0",
            "--scale",
            &self.shared.config.d2_scale,
            "--theme",
            &style.theme,
        ])
        .run()?;

        self.load_image(snippet, &output_path)
    }

    fn do_render_typst(
        &self,
        snippet: ImageSnippet,
        input: &str,
        style: &TypstStyle,
    ) -> Result<Image, ThirdPartyRenderError> {
        let workdir = tempfile::Builder::default().prefix(".presenterm").tempdir_in(&self.shared.root_dir)?;
        let mut typst_input = Self::generate_page_header(style)?;
        typst_input.push_str(input);

        let input_path = workdir.path().join("input.typst");
        fs::write(&input_path, &typst_input)?;

        let output_path = workdir.path().join("output.png");
        ThirdPartyTools::typst(&[
            "compile",
            "--format",
            "png",
            "--root",
            &self.shared.root_dir,
            "--ppi",
            &self.shared.config.typst_ppi,
            &input_path.to_string_lossy(),
            &output_path.to_string_lossy(),
        ])
        .run()?;

        self.load_image(snippet, &output_path)
    }

    fn generate_page_header(style: &TypstStyle) -> Result<String, ThirdPartyRenderError> {
        let x_margin = style.horizontal_margin;
        let y_margin = style.vertical_margin;
        let background = style
            .style
            .colors
            .background
            .as_ref()
            .map(Self::as_typst_color)
            .unwrap_or_else(|| Ok(String::from("none")))?;
        let mut header = format!(
            "#set page(width: auto, height: auto, margin: (x: {x_margin}pt, y: {y_margin}pt), fill: {background})\n"
        );
        if let Some(color) = &style.style.colors.foreground {
            let color = Self::as_typst_color(color)?;
            header.push_str(&format!("#set text(fill: {color})\n"));
        }
        Ok(header)
    }

    fn as_typst_color(color: &Color) -> Result<String, ThirdPartyRenderError> {
        match color.as_rgb() {
            Some((r, g, b)) => Ok(format!("rgb(\"#{r:02x}{g:02x}{b:02x}\")")),
            None => Err(ThirdPartyRenderError::UnsupportedColor(RawColor::from(*color).to_string())),
        }
    }

    fn load_image(&self, snippet: ImageSnippet, path: &Path) -> Result<Image, ThirdPartyRenderError> {
        let contents = fs::read(path)?;
        let image = image::load_from_memory(&contents)?;
        let image = self.state.lock().unwrap().image_registry.register(ImageSpec::Generated(image))?;
        self.state.lock().unwrap().cache.insert(snippet, image.clone());
        Ok(image)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThirdPartyRenderError {
    #[error(transparent)]
    Execution(#[from] ExecutionError),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid image: {0}")]
    InvalidImage(#[from] image::ImageError),

    #[error("invalid image: {0}")]
    RegisterImage(#[from] RegisterImageError),

    #[error("unsupported color '{0}', only RGB is supported")]
    UnsupportedColor(String),
}

#[derive(Hash, PartialEq, Eq)]
enum SnippetSource {
    Typst,
    Latex,
    Mermaid,
    D2,
}

#[derive(Hash, PartialEq, Eq)]
struct ImageSnippet {
    snippet: String,
    source: SnippetSource,
}

#[derive(Debug)]
pub(crate) struct RenderThirdParty {
    contents: Arc<Mutex<Option<Output>>>,
    pending_result: Arc<Mutex<RenderResult>>,
    default_style: TextStyle,
    width: Option<Percent>,
}

impl RenderThirdParty {
    fn new(pending_result: Arc<Mutex<RenderResult>>, default_style: TextStyle, width: Option<Percent>) -> Self {
        Self { contents: Default::default(), pending_result, default_style, width }
    }
}

impl RenderAsync for RenderThirdParty {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(OperationPollable { contents: self.contents.clone(), pending_result: self.pending_result.clone() })
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        RenderAsyncStartPolicy::Automatic
    }
}

impl AsRenderOperations for RenderThirdParty {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        match &*self.contents.lock().unwrap() {
            Some(Output::Image(image)) => {
                let size = match &self.width {
                    Some(percent) => ImageSize::WidthScaled { ratio: percent.as_ratio() },
                    None => Default::default(),
                };
                let properties = ImageRenderProperties {
                    size,
                    background_color: self.default_style.colors.background,
                    ..Default::default()
                };

                vec![RenderOperation::RenderImage(image.clone(), properties)]
            }
            Some(Output::Error) => Vec::new(),
            None => {
                let text = Line::from(Text::new("Loading...", TextStyle::default().bold()));
                vec![RenderOperation::RenderText {
                    line: text.into(),
                    alignment: Alignment::Center { minimum_margin: Default::default(), minimum_size: 0 },
                }]
            }
        }
    }
}

#[derive(Debug)]
enum Output {
    Image(Image),
    Error,
}

#[derive(Clone)]
struct OperationPollable {
    contents: Arc<Mutex<Option<Output>>>,
    pending_result: Arc<Mutex<RenderResult>>,
}

impl Pollable for OperationPollable {
    fn poll(&mut self) -> PollableState {
        let mut contents = self.contents.lock().unwrap();
        if contents.is_some() {
            return PollableState::Done;
        }
        match mem::take(&mut *self.pending_result.lock().unwrap()) {
            RenderResult::Success(image) => {
                *contents = Some(Output::Image(image));
                PollableState::Done
            }
            RenderResult::Failure(error) => {
                *contents = Some(Output::Error);
                PollableState::Failed { error }
            }
            RenderResult::Pending => PollableState::Unmodified,
        }
    }
}
