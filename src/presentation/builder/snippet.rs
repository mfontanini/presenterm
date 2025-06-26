use super::{BuildError, BuildResult};
use crate::{
    code::{
        execute::LanguageSnippetExecutor,
        snippet::{
            ExternalFile, Highlight, HighlightContext, HighlightGroup, HighlightMutator, HighlightedLine, Snippet,
            SnippetExec, SnippetExecutorSpec, SnippetLanguage, SnippetLine, SnippetParser, SnippetRepr,
            SnippetSplitter,
        },
    },
    markdown::elements::SourcePosition,
    presentation::builder::{PresentationBuilder, error::InvalidPresentation},
    render::{
        operation::{AsRenderOperations, RenderAsyncStartPolicy, RenderOperation},
        properties::WindowSize,
    },
    theme::{Alignment, CodeBlockStyle},
    third_party::ThirdPartyRenderRequest,
    ui::execution::{
        RunAcquireTerminalSnippet, RunImageSnippet, RunSnippetOperation, SnippetExecutionDisabledOperation,
        disabled::ExecutionType, snippet::DisplaySeparator, validator::ValidateSnippetOperation,
    },
};
use itertools::Itertools;
use std::{cell::RefCell, rc::Rc};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_code(&mut self, info: String, code: String, source_position: SourcePosition) -> BuildResult {
        let mut snippet = SnippetParser::parse(info, code)
            .map_err(|e| self.invalid_presentation(source_position, InvalidPresentation::Snippet(e.to_string())))?;
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
                    // TODO: representation and execution should probably be merged
                    let SnippetExec::Exec(spec) = snippet.attributes.execution.clone() else {
                        panic!("not an exec snippet");
                    };
                    return self.push_code_execution(snippet, 0, ExecutionMode::ReplaceSnippet, &spec);
                }
            }
            SnippetRepr::Snippet => (),
        };

        let block_length = self.push_code_lines(&snippet);
        match snippet.attributes.execution.clone() {
            SnippetExec::None => Ok(()),
            SnippetExec::Exec(_) | SnippetExec::AcquireTerminal(_) if !execution_allowed => {
                let exec_type = match snippet.attributes.representation {
                    SnippetRepr::Image => ExecutionType::Image,
                    SnippetRepr::ExecReplace => ExecutionType::ExecReplace,
                    SnippetRepr::Render | SnippetRepr::Snippet => ExecutionType::Execute,
                };
                self.push_execution_disabled_operation(exec_type);
                Ok(())
            }
            SnippetExec::Exec(spec) => {
                self.push_code_execution(snippet, block_length, ExecutionMode::AlongSnippet, &spec)
            }
            SnippetExec::AcquireTerminal(spec) => self.push_acquire_terminal_execution(snippet, block_length, &spec),
            SnippetExec::Validate(spec) => {
                let executor = self.snippet_executor.language_executor(&snippet.language, &spec)?;
                self.push_validator(&snippet, &executor);
                Ok(())
            }
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
        let block_length = lines.iter().map(|line| line.width()).max().unwrap_or(0) * self.slide_font_size() as usize;
        let block_length = block_length as u16;
        let (lines, context) = self.highlight_lines(snippet, lines, block_length);
        for line in lines {
            self.chunk_operations.push(RenderOperation::RenderDynamic(Rc::new(line)));
        }
        self.chunk_operations.push(RenderOperation::SetColors(self.theme.default_style.style.colors));
        if self.options.allow_mutations && context.borrow().groups.len() > 1 {
            self.chunk_mutators.push(Box::new(HighlightMutator::new(context)));
        }
        block_length
    }

    fn load_external_snippet(
        &mut self,
        mut code: Snippet,
        source_position: SourcePosition,
    ) -> Result<Snippet, BuildError> {
        let file: ExternalFile = serde_yaml::from_str(&code.contents)
            .map_err(|e| self.invalid_presentation(source_position, InvalidPresentation::Snippet(e.to_string())))?;
        let path = file.path;
        let base_path = self.resource_base_path();
        let contents = self.resources.external_text_file(&path, &base_path).map_err(|e| {
            self.invalid_presentation(
                source_position,
                InvalidPresentation::Snippet(format!("failed to load snippet {path:?}: {e}")),
            )
        })?;
        code.language = file.language;
        code.contents = Self::filter_lines(contents, file.start_line, file.end_line);
        Ok(code)
    }

    fn filter_lines(code: String, start: Option<usize>, end: Option<usize>) -> String {
        let start = start.map(|s| s.saturating_sub(1));
        match (start, end) {
            (None, None) => code,
            (None, Some(end)) => code.lines().take(end).join("\n"),
            (Some(start), None) => code.lines().skip(start).join("\n"),
            (Some(start), Some(end)) => code.lines().skip(start).take(end.saturating_sub(start)).join("\n"),
        }
    }

    fn push_rendered_code(&mut self, code: Snippet, source_position: SourcePosition) -> BuildResult {
        let Snippet { contents, language, attributes } = code;
        let request = match language {
            SnippetLanguage::Typst => ThirdPartyRenderRequest::Typst(contents, self.theme.typst.clone()),
            SnippetLanguage::Latex => ThirdPartyRenderRequest::Latex(contents, self.theme.typst.clone()),
            SnippetLanguage::Mermaid => ThirdPartyRenderRequest::Mermaid(contents, self.theme.mermaid.clone()),
            SnippetLanguage::D2 => ThirdPartyRenderRequest::D2(contents, self.theme.d2.clone()),
            _ => {
                return Err(self.invalid_presentation(
                    source_position,
                    InvalidPresentation::Snippet(format!("language {language:?} doesn't support rendering")),
                ));
            }
        };
        let operation = self.third_party.render(request, &self.theme, attributes.width)?;
        self.chunk_operations.push(operation);
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
        let font_size = self.slide_font_size();
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
                right_padding_length: line.right_padding_length * font_size as u16,
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
        self.chunk_operations.push(RenderOperation::RenderAsync(Rc::new(operation)));
    }

    fn push_code_as_image(&mut self, snippet: Snippet) -> BuildResult {
        let executor = self.snippet_executor.language_executor(&snippet.language, &Default::default())?;
        self.push_validator(&snippet, &executor);

        let operation = RunImageSnippet::new(
            snippet,
            executor,
            self.image_registry.clone(),
            self.theme.execution_output.status.clone(),
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.chunk_operations.push(operation);
        Ok(())
    }

    fn push_acquire_terminal_execution(
        &mut self,
        snippet: Snippet,
        block_length: u16,
        spec: &SnippetExecutorSpec,
    ) -> BuildResult {
        let executor = self.snippet_executor.language_executor(&snippet.language, spec)?;
        let block_length = self.theme.code.alignment.adjust_size(block_length);
        let operation = RunAcquireTerminalSnippet::new(
            snippet,
            executor,
            self.theme.execution_output.status.clone(),
            block_length,
            self.slide_font_size(),
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.chunk_operations.push(operation);
        Ok(())
    }

    fn push_code_execution(
        &mut self,
        snippet: Snippet,
        block_length: u16,
        mode: ExecutionMode,
        spec: &SnippetExecutorSpec,
    ) -> BuildResult {
        let executor = self.snippet_executor.language_executor(&snippet.language, spec)?;
        self.push_validator(&snippet, &executor);

        let separator = match mode {
            ExecutionMode::AlongSnippet => DisplaySeparator::On,
            ExecutionMode::ReplaceSnippet => DisplaySeparator::Off,
        };
        let default_alignment = self.code_style(&snippet).alignment;
        // If we're replacing the snippet output and we have center alignment, use center alignment but
        // without any margins and minimum sizes so we truly center the output.
        let alignment = match (&mode, default_alignment) {
            (ExecutionMode::ReplaceSnippet, Alignment::Center { .. }) => {
                Alignment::Center { minimum_margin: Default::default(), minimum_size: 0 }
            }
            (_, alignment) => alignment,
        };
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
            executor,
            default_colors,
            execution_output_style,
            block_length,
            separator,
            alignment,
            self.slide_font_size(),
            policy,
            self.theme.execution_output.padding.clone(),
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.chunk_operations.push(operation);
        Ok(())
    }

    fn push_differ(&mut self, text: String) {
        self.chunk_operations.push(RenderOperation::RenderDynamic(Rc::new(Differ(text))));
    }

    fn push_validator(&mut self, snippet: &Snippet, executor: &LanguageSnippetExecutor) {
        if !self.options.validate_snippets {
            return;
        }
        let operation = ValidateSnippetOperation::new(snippet.clone(), executor.clone());
        self.chunk_operations.push(RenderOperation::RenderAsync(Rc::new(operation)));
    }
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

#[derive(Debug)]
enum ExecutionMode {
    AlongSnippet,
    ReplaceSnippet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::no_filters(None, None, &["a", "b", "c", "d", "e"])]
    #[case::start_from_first(Some(1), None, &["a", "b", "c", "d", "e"])]
    #[case::start_from_second(Some(2), None, &["b", "c", "d", "e"])]
    #[case::start_from_end(Some(5), None, &["e"])]
    #[case::start_from_past_end(Some(6), None, &[])]
    #[case::end_last(None, Some(5), &["a", "b", "c", "d", "e"])]
    #[case::end_one_before_last(None, Some(4), &["a", "b", "c", "d"])]
    #[case::end_at_first(None, Some(1), &["a"])]
    #[case::end_at_zero(None, Some(0), &[])]
    #[case::start_and_end(Some(2), Some(3), &["b", "c"])]
    #[case::crossed(Some(2), Some(1), &[])]
    fn filter_lines(#[case] start: Option<usize>, #[case] end: Option<usize>, #[case] expected: &[&str]) {
        let code = ["a", "b", "c", "d", "e"].join("\n");
        let output = PresentationBuilder::filter_lines(code, start, end);
        let expected = expected.join("\n");
        assert_eq!(output, expected);
    }
}
