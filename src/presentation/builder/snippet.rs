use super::{BuildError, BuildResult, ExecutionMode, PresentationBuilderOptions};
use crate::{
    ImageRegistry,
    code::{
        execute::SnippetExecutor,
        highlighting::SnippetHighlighter,
        snippet::{
            ExternalFile, Highlight, HighlightContext, HighlightGroup, HighlightMutator, HighlightedLine, Snippet,
            SnippetExec, SnippetLanguage, SnippetLine, SnippetParser, SnippetRepr, SnippetSplitter,
        },
    },
    markdown::elements::SourcePosition,
    presentation::ChunkMutator,
    render::{
        operation::{AsRenderOperations, RenderAsyncStartPolicy, RenderOperation},
        properties::WindowSize,
    },
    resource::Resources,
    theme::{CodeBlockStyle, PresentationTheme},
    third_party::{ThirdPartyRender, ThirdPartyRenderRequest},
    ui::execution::{
        RunAcquireTerminalSnippet, RunImageSnippet, RunSnippetOperation, SnippetExecutionDisabledOperation,
        disabled::ExecutionType, snippet::DisplaySeparator,
    },
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(crate) struct SnippetProcessorState<'a> {
    pub(crate) resources: &'a Resources,
    pub(crate) image_registry: &'a ImageRegistry,
    pub(crate) snippet_executor: Arc<SnippetExecutor>,
    pub(crate) theme: &'a PresentationTheme,
    pub(crate) third_party: &'a ThirdPartyRender,
    pub(crate) highlighter: &'a SnippetHighlighter,
    pub(crate) options: &'a PresentationBuilderOptions,
    pub(crate) font_size: u8,
}

pub(crate) struct SnippetProcessor<'a> {
    operations: Vec<RenderOperation>,
    mutators: Vec<Box<dyn ChunkMutator>>,
    resources: &'a Resources,
    image_registry: &'a ImageRegistry,
    snippet_executor: Arc<SnippetExecutor>,
    theme: &'a PresentationTheme,
    third_party: &'a ThirdPartyRender,
    highlighter: &'a SnippetHighlighter,
    options: &'a PresentationBuilderOptions,
    font_size: u8,
}

impl<'a> SnippetProcessor<'a> {
    pub(crate) fn new(state: SnippetProcessorState<'a>) -> Self {
        let SnippetProcessorState {
            resources,
            image_registry,
            snippet_executor,
            theme,
            third_party,
            highlighter,
            options,
            font_size,
        } = state;
        Self {
            operations: Vec::new(),
            mutators: Vec::new(),
            resources,
            image_registry,
            snippet_executor,
            theme,
            third_party,
            highlighter,
            options,
            font_size,
        }
    }

    pub(crate) fn process_code(
        mut self,
        info: String,
        code: String,
        source_position: SourcePosition,
    ) -> Result<SnippetOperations, BuildError> {
        self.do_process_code(info, code, source_position)?;

        let Self { operations, mutators, .. } = self;
        Ok(SnippetOperations { operations, mutators })
    }

    fn do_process_code(&mut self, info: String, code: String, source_position: SourcePosition) -> BuildResult {
        let mut snippet = SnippetParser::parse(info, code)
            .map_err(|e| BuildError::InvalidSnippet { source_position, error: e.to_string() })?;
        if matches!(snippet.language, SnippetLanguage::File) {
            snippet = self.load_external_snippet(snippet, source_position)?;
        }
        if self.options.auto_render_languages.contains(&snippet.language) {
            snippet.attributes.representation = SnippetRepr::Render;
        }
        self.push_differ(snippet.contents.clone());
        // Redraw slide if attributes change
        self.push_differ(format!("{:?}", snippet.attributes));

        let execution_allowed = self.is_execution_allowed(&snippet);
        match snippet.attributes.representation {
            SnippetRepr::Render => return self.push_rendered_code(snippet, source_position),
            SnippetRepr::Image => {
                if execution_allowed {
                    return self.push_code_as_image(snippet);
                }
            }
            SnippetRepr::ExecReplace => {
                if execution_allowed {
                    return self.push_code_execution(snippet, 0, ExecutionMode::ReplaceSnippet);
                }
            }
            SnippetRepr::Snippet => (),
        };

        let block_length = self.push_code_lines(&snippet);
        match snippet.attributes.execution {
            SnippetExec::None => Ok(()),
            SnippetExec::Exec | SnippetExec::AcquireTerminal if !execution_allowed => {
                let exec_type = match snippet.attributes.representation {
                    SnippetRepr::Image => ExecutionType::Image,
                    SnippetRepr::ExecReplace => ExecutionType::ExecReplace,
                    SnippetRepr::Render | SnippetRepr::Snippet => ExecutionType::Execute,
                };
                self.push_execution_disabled_operation(exec_type);
                Ok(())
            }
            SnippetExec::Exec => self.push_code_execution(snippet, block_length, ExecutionMode::AlongSnippet),
            SnippetExec::AcquireTerminal => self.push_acquire_terminal_execution(snippet, block_length),
        }
    }

    fn is_execution_allowed(&self, snippet: &Snippet) -> bool {
        match snippet.attributes.representation {
            SnippetRepr::Snippet => self.options.enable_snippet_execution,
            SnippetRepr::Image | SnippetRepr::ExecReplace => self.options.enable_snippet_execution_replace,
            SnippetRepr::Render => true,
        }
    }

    fn push_code_lines(&mut self, snippet: &Snippet) -> u16 {
        let lines = SnippetSplitter::new(&self.theme.code, self.snippet_executor.hidden_line_prefix(&snippet.language))
            .split(snippet);
        let block_length = lines.iter().map(|line| line.width()).max().unwrap_or(0) * self.font_size as usize;
        let block_length = block_length as u16;
        let (lines, context) = self.highlight_lines(snippet, lines, block_length);
        for line in lines {
            self.operations.push(RenderOperation::RenderDynamic(Rc::new(line)));
        }
        self.operations.push(RenderOperation::SetColors(self.theme.default_style.style.colors));
        if self.options.allow_mutations && context.borrow().groups.len() > 1 {
            self.mutators.push(Box::new(HighlightMutator::new(context)));
        }
        block_length
    }

    fn load_external_snippet(
        &mut self,
        mut code: Snippet,
        source_position: SourcePosition,
    ) -> Result<Snippet, BuildError> {
        let file: ExternalFile = serde_yaml::from_str(&code.contents)
            .map_err(|e| BuildError::InvalidSnippet { source_position, error: e.to_string() })?;
        let path = file.path;
        let path_display = path.display();
        let contents = self.resources.external_snippet(&path).map_err(|e| BuildError::InvalidSnippet {
            source_position,
            error: format!("failed to load {path_display}: {e}"),
        })?;
        code.language = file.language;
        code.contents = contents;
        Ok(code)
    }

    fn push_rendered_code(&mut self, code: Snippet, source_position: SourcePosition) -> BuildResult {
        let Snippet { contents, language, attributes } = code;
        let request = match language {
            SnippetLanguage::Typst => ThirdPartyRenderRequest::Typst(contents, self.theme.typst.clone()),
            SnippetLanguage::Latex => ThirdPartyRenderRequest::Latex(contents, self.theme.typst.clone()),
            SnippetLanguage::Mermaid => ThirdPartyRenderRequest::Mermaid(contents, self.theme.mermaid.clone()),
            _ => {
                return Err(BuildError::InvalidSnippet {
                    source_position,
                    error: format!("language {language:?} doesn't support rendering"),
                })?;
            }
        };
        let operation = self.third_party.render(request, self.theme, attributes.width)?;
        self.operations.push(operation);
        Ok(())
    }

    fn highlight_lines(
        &self,
        code: &Snippet,
        lines: Vec<SnippetLine>,
        block_length: u16,
    ) -> (Vec<HighlightedLine>, Rc<RefCell<HighlightContext>>) {
        let mut code_highlighter = self.highlighter.language_highlighter(&code.language);
        let style = self.code_style(code);
        let block_length = self.theme.code.alignment.adjust_size(block_length);
        let font_size = self.font_size;
        let dim_style = {
            let mut highlighter = self.highlighter.language_highlighter(&SnippetLanguage::Rust);
            highlighter.style_line("//", &style).0.first().expect("no styles").style.size(font_size)
        };
        let groups = match self.options.allow_mutations {
            true => code.attributes.highlight_groups.clone(),
            false => vec![HighlightGroup::new(vec![Highlight::All])],
        };
        let context =
            Rc::new(RefCell::new(HighlightContext { groups, current: 0, block_length, alignment: style.alignment }));

        let mut output = Vec::new();
        for line in lines.into_iter() {
            let prefix = line.dim_prefix(&dim_style);
            let highlighted = line.highlight(&mut code_highlighter, &style, font_size);
            let not_highlighted = line.dim(&dim_style);
            let line_number = line.line_number;
            let context = context.clone();
            output.push(HighlightedLine {
                prefix,
                right_padding_length: line.right_padding_length * self.font_size as u16,
                highlighted,
                not_highlighted,
                line_number,
                context,
                block_color: dim_style.colors.background,
            });
        }
        (output, context)
    }

    fn code_style(&self, snippet: &Snippet) -> CodeBlockStyle {
        let mut style = self.theme.code.clone();
        if snippet.attributes.no_background {
            style.background = false;
        }
        style
    }

    fn push_execution_disabled_operation(&mut self, exec_type: ExecutionType) {
        let policy = match exec_type {
            ExecutionType::ExecReplace | ExecutionType::Image => RenderAsyncStartPolicy::Automatic,
            ExecutionType::Execute => RenderAsyncStartPolicy::OnDemand,
        };
        let operation = SnippetExecutionDisabledOperation::new(
            self.theme.execution_output.status.failure_style,
            self.theme.code.alignment,
            policy,
            exec_type,
        );
        self.operations.push(RenderOperation::RenderAsync(Rc::new(operation)));
    }

    fn push_code_as_image(&mut self, snippet: Snippet) -> BuildResult {
        if !self.snippet_executor.is_execution_supported(&snippet.language) {
            return Err(BuildError::UnsupportedExecution(snippet.language));
        }
        let operation = RunImageSnippet::new(
            snippet,
            self.snippet_executor.clone(),
            self.image_registry.clone(),
            self.theme.execution_output.status.clone(),
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.operations.push(operation);
        Ok(())
    }

    fn push_acquire_terminal_execution(&mut self, snippet: Snippet, block_length: u16) -> BuildResult {
        if !self.snippet_executor.is_execution_supported(&snippet.language) {
            return Err(BuildError::UnsupportedExecution(snippet.language));
        }
        let block_length = self.theme.code.alignment.adjust_size(block_length);
        let operation = RunAcquireTerminalSnippet::new(
            snippet,
            self.snippet_executor.clone(),
            self.theme.execution_output.status.clone(),
            block_length,
            self.font_size,
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.operations.push(operation);
        Ok(())
    }

    fn push_code_execution(&mut self, snippet: Snippet, block_length: u16, mode: ExecutionMode) -> BuildResult {
        if !self.snippet_executor.is_execution_supported(&snippet.language) {
            return Err(BuildError::UnsupportedExecution(snippet.language));
        }
        let separator = match mode {
            ExecutionMode::AlongSnippet => DisplaySeparator::On,
            ExecutionMode::ReplaceSnippet => DisplaySeparator::Off,
        };
        let alignment = self.code_style(&snippet).alignment;
        let default_colors = self.theme.default_style.style.colors;
        let mut execution_output_style = self.theme.execution_output.clone();
        if snippet.attributes.no_background {
            execution_output_style.style.colors.background = None;
        }
        let policy = match mode {
            ExecutionMode::AlongSnippet => RenderAsyncStartPolicy::OnDemand,
            ExecutionMode::ReplaceSnippet => RenderAsyncStartPolicy::Automatic,
        };
        let operation = RunSnippetOperation::new(
            snippet,
            self.snippet_executor.clone(),
            default_colors,
            execution_output_style,
            block_length,
            separator,
            alignment,
            self.font_size,
            policy,
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.operations.push(operation);
        Ok(())
    }

    fn push_differ(&mut self, text: String) {
        self.operations.push(RenderOperation::RenderDynamic(Rc::new(Differ(text))));
    }
}

pub(crate) struct SnippetOperations {
    pub(crate) operations: Vec<RenderOperation>,
    pub(crate) mutators: Vec<Box<dyn ChunkMutator>>,
}

#[derive(Debug)]
struct Differ(String);

impl AsRenderOperations for Differ {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        Vec::new()
    }

    fn diffable_content(&self) -> Option<&str> {
        Some(&self.0)
    }
}
