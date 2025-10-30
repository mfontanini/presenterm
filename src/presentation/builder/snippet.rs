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
        RunAcquireTerminalSnippet, RunImageSnippet, SnippetExecutionDisabledOperation, SnippetOutputOperation,
        disabled::ExecutionType,
        output::{ExecIndicator, ExecIndicatorStyle, RunSnippetTrigger, SnippetHandle},
        validator::ValidateSnippetOperation,
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
        if self.theme.code.line_numbers {
            snippet.attributes.line_numbers = true;
        }
        if self.options.auto_render_languages.contains(&snippet.language) {
            snippet.attributes.representation = SnippetRepr::Render;
        }
        // Ids can only be used in `+exec` snippets.
        if snippet.attributes.id.is_some()
            && (!matches!(snippet.attributes.execution, SnippetExec::Exec(_))
                || !matches!(snippet.attributes.representation, SnippetRepr::Snippet))
        {
            return Err(self.invalid_presentation(source_position, InvalidPresentation::SnippetIdNonExec));
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
                    return self.push_replace_code_execution(snippet.clone());
                }
            }
            SnippetRepr::Snippet => (),
        };

        let block_length = self.push_code_lines(&snippet);
        match snippet.attributes.execution.clone() {
            SnippetExec::None => Ok(()),
            SnippetExec::Exec(_) | SnippetExec::AutoExec(_) | SnippetExec::AcquireTerminal(_) if !execution_allowed => {
                let mut exec_type = match snippet.attributes.representation {
                    SnippetRepr::Image => ExecutionType::Image,
                    SnippetRepr::ExecReplace => ExecutionType::ExecReplace,
                    SnippetRepr::Render | SnippetRepr::Snippet => ExecutionType::Execute,
                };
                if matches!(snippet.attributes.execution, SnippetExec::AutoExec(_)) {
                    exec_type = ExecutionType::ExecReplace;
                }
                self.push_execution_disabled_operation(exec_type);
                Ok(())
            }
            SnippetExec::Exec(spec) | SnippetExec::AutoExec(spec) => {
                let policy = if matches!(snippet.attributes.execution, SnippetExec::AutoExec(_)) {
                    RenderAsyncStartPolicy::Automatic
                } else {
                    RenderAsyncStartPolicy::OnDemand
                };
                let executor = self.snippet_executor.language_executor(&snippet.language, &spec)?;
                let alignment = self.code_style(&snippet).alignment;
                let handle = SnippetHandle::new(snippet.clone(), executor, policy);
                self.chunk_operations
                    .push(RenderOperation::RenderAsync(Rc::new(RunSnippetTrigger::new(handle.clone()))));
                self.push_indicator(handle.clone(), block_length, alignment);
                match snippet.attributes.id.clone() {
                    Some(id) => {
                        if self.executable_snippets.insert(id.clone(), handle).is_some() {
                            return Err(self
                                .invalid_presentation(source_position, InvalidPresentation::SnippetAlreadyExists(id)));
                        }
                        Ok(())
                    }
                    None => {
                        self.push_line_break();
                        self.push_code_execution(block_length, handle, alignment)
                    }
                }
            }
            SnippetExec::AcquireTerminal(spec) => self.push_acquire_terminal_execution(snippet, block_length, &spec),
            SnippetExec::Validate(spec) => {
                let executor = self.snippet_executor.language_executor(&snippet.language, &spec)?;
                self.push_validator(&snippet, &executor);
                Ok(())
            }
        }
    }

    pub(crate) fn push_detached_code_execution(&mut self, handle: SnippetHandle) -> BuildResult {
        let alignment = self.code_style(&handle.snippet()).alignment;
        self.push_code_execution(0, handle, alignment)
    }

    fn is_execution_allowed(&self, snippet: &Snippet) -> bool {
        match snippet.attributes.representation {
            SnippetRepr::Snippet => match snippet.attributes.execution {
                SnippetExec::AutoExec(_) => self.options.enable_snippet_execution_replace,
                _ => self.options.enable_snippet_execution,
            },
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

    fn push_replace_code_execution(&mut self, snippet: Snippet) -> BuildResult {
        // TODO: representation and execution should probably be merged
        let SnippetExec::Exec(spec) = snippet.attributes.execution.clone() else {
            panic!("not an exec snippet");
        };
        let alignment = match self.code_style(&snippet).alignment {
            // If we're replacing the snippet output, we have center alignment and no background, use
            // center alignment but without any margins and minimum sizes so we truly center the output.
            Alignment::Center { .. } if snippet.attributes.no_background => {
                Alignment::Center { minimum_margin: Default::default(), minimum_size: 0 }
            }
            other => other,
        };
        let executor = self.snippet_executor.language_executor(&snippet.language, &spec)?;
        let handle = SnippetHandle::new(snippet, executor, RenderAsyncStartPolicy::Automatic);
        self.chunk_operations.push(RenderOperation::RenderAsync(Rc::new(RunSnippetTrigger::new(handle.clone()))));
        self.push_code_execution(0, handle, alignment)
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

        let operation =
            RunImageSnippet::new(snippet, executor, self.image_registry.clone(), self.theme.execution_output.status);
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
            self.theme.execution_output.status,
            block_length,
            self.slide_font_size(),
        );
        let operation = RenderOperation::RenderAsync(Rc::new(operation));
        self.chunk_operations.push(operation);
        Ok(())
    }

    fn push_indicator(&mut self, handle: SnippetHandle, block_length: u16, alignment: Alignment) {
        let style = ExecIndicatorStyle {
            theme: self.theme.execution_output.status,
            block_length,
            font_size: self.slide_font_size(),
            alignment,
        };
        let indicator = Rc::new(ExecIndicator::new(handle, style));
        self.chunk_operations.push(RenderOperation::RenderDynamic(indicator));
    }

    fn push_code_execution(&mut self, block_length: u16, handle: SnippetHandle, alignment: Alignment) -> BuildResult {
        let executor = handle.executor();
        let snippet = handle.snippet();
        self.push_validator(&snippet, &executor);

        let default_colors = self.theme.default_style.style.colors;
        let mut execution_output_style = self.theme.execution_output.clone();
        if snippet.attributes.no_background {
            execution_output_style.style.colors.background = None;
            execution_output_style.padding = Default::default();
        }
        let operation = SnippetOutputOperation::new(
            handle,
            default_colors,
            execution_output_style,
            block_length,
            alignment,
            self.slide_font_size(),
        );
        let operation = RenderOperation::RenderDynamic(Rc::new(operation));
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{
        markdown::text_style::Color,
        presentation::builder::utils::{RunAsyncRendersPolicy, Test},
        theme::raw,
    };
    use rstest::rstest;
    use std::fs;

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

    #[test]
    fn plain() {
        let input = "
```bash
echo hi
```";
        let lines = Test::new(input).render().rows(3).columns(7).into_lines();
        let expected = &["       ", "echo hi", "       "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn external_snippet() {
        let temp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
        let path = temp.path();
        fs::write(path, "echo hi").unwrap();

        let path = path.to_string_lossy();
        let input = format!(
            "
```file
path: {path}
language: bash
```
"
        );
        let lines = Test::new(input).render().rows(3).columns(7).into_lines();
        let expected = &["       ", "echo hi", "       "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn line_numbers() {
        let input = "
```bash +line_numbers
hi
bye
```";
        let lines = Test::new(input).render().rows(4).columns(5).into_lines();
        let expected = &["     ", "1 hi ", "2 bye", "     "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn line_numbers_via_theme() {
        let input = "---
theme:
  override:
    code:
      line_numbers: true
---

```bash
hi
bye
```";
        let lines = Test::new(input).render().rows(4).columns(5).into_lines();
        let expected = &["     ", "1 hi ", "2 bye", "     "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn surroundings() {
        let input = "
---
```bash
echo hi
```
---";
        let lines = Test::new(input).render().rows(7).columns(7).into_lines();
        let expected = &[
            //
            "       ",
            "———————",
            "       ",
            "echo hi",
            "       ",
            "———————",
            "       ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn padding() {
        let input = "
```bash
echo hi
```";
        let theme = raw::PresentationTheme {
            code: raw::CodeBlockStyle {
                padding: raw::PaddingRect { horizontal: Some(2), vertical: Some(1) },
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(5).columns(13).into_lines();
        let expected = &[
            //
            "             ",
            "             ",
            "  echo hi    ",
            "             ",
            "             ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_no_run() {
        let input = "
```bash +exec
echo hi
```";
        let lines =
            Test::new(input).render().rows(4).columns(19).run_async_renders(RunAsyncRendersPolicy::None).into_lines();
        let expected = &[
            //
            "                   ",
            "echo hi            ",
            "                   ",
            "—— [not started] ——",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_auto() {
        let input = "
```bash +auto_exec
echo hi
```";
        let lines = Test::new(input)
            .render()
            .rows(6)
            .columns(19)
            .run_async_renders(RunAsyncRendersPolicy::OnlyAutomatic)
            .into_lines();
        let expected = &[
            //
            "                   ",
            "echo hi            ",
            "                   ",
            "——— [finished] ————",
            "                   ",
            "hi                 ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn validate() {
        let input = "
```bash +validate
echo hi
```";
        let lines =
            Test::new(input).render().rows(4).columns(19).run_async_renders(RunAsyncRendersPolicy::None).into_lines();
        let expected = &["                   ", "echo hi            ", "                   ", "                   "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_disabled() {
        let input = "
```bash +exec
echo hi
```";
        let lines = Test::new(input).disable_exec().render().rows(6).columns(25).into_lines();
        let expected = &[
            "                         ",
            "echo hi                  ",
            "                         ",
            "snippet +exec is         ",
            "disabled, run with -x to ",
            "enable                   ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_replace_disabled() {
        let input = "
```bash +exec_replace
echo hi
```";
        let lines = Test::new(input).disable_exec_replace().render().rows(6).columns(25).into_lines();
        let expected = &[
            "                         ",
            "echo hi                  ",
            "                         ",
            "snippet +exec_replace is ",
            "disabled, run with -X to ",
            "enable                   ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec() {
        let input = "
```bash +exec
echo hi
```";
        let theme = raw::PresentationTheme {
            execution_output: raw::ExecutionOutputBlockStyle {
                colors: raw::RawColors {
                    background: Some(raw::RawColor::Color(Color::new(45, 45, 45))),
                    foreground: None,
                },
                padding: raw::PaddingRect { horizontal: Some(1), vertical: Some(1) },
                ..Default::default()
            },
            ..Default::default()
        };
        let (lines, styles) = Test::new(input)
            .theme(theme)
            .render()
            .map_background(Color::new(45, 45, 45), 'x')
            .rows(8)
            .columns(16)
            .into_parts();
        let expected_lines = &[
            "                ",
            "echo hi         ",
            "                ",
            "—— [finished] ——",
            "                ",
            "                ",
            " hi             ",
            "                ",
        ];
        let expected_styles = &[
            "                ",
            "xxxxxxxxxxxxxxxx",
            "                ",
            "                ",
            "                ",
            "xxxxxxxxxxxxxxxx",
            "xxxxxxxxxxxxxxxx",
            "xxxxxxxxxxxxxxxx",
        ];
        assert_eq!(lines, expected_lines);
        assert_eq!(styles, expected_styles);
    }

    #[test]
    fn exec_font_size() {
        let input = "
<!-- font_size: 2 -->
```bash +exec
echo hi
```";
        let lines = Test::new(input).render().rows(8).columns(32).into_lines();
        let expected = &[
            "                                ",
            "e c h o   h i                   ",
            "                                ",
            "                                ",
            "— —   [ f i n i s h e d ]   — — ",
            "                                ",
            "                                ",
            "h i                             ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_font_size_centered() {
        let input = "
<!-- font_size: 2 -->
```bash +exec
echo hi
```";
        let theme = raw::PresentationTheme {
            code: raw::CodeBlockStyle {
                alignment: Some(raw::Alignment::Center { minimum_margin: raw::Margin::Fixed(0), minimum_size: 40 }),
                ..Default::default()
            },
            execution_output: raw::ExecutionOutputBlockStyle {
                colors: raw::RawColors {
                    background: Some(raw::RawColor::Color(Color::new(45, 45, 45))),
                    foreground: None,
                },
                padding: raw::PaddingRect { horizontal: Some(1), vertical: Some(1) },
                ..Default::default()
            },
            ..Default::default()
        };
        let (lines, styles) = Test::new(input)
            .theme(theme)
            .render()
            .map_background(Color::new(45, 45, 45), 'x')
            .rows(10)
            .columns(40)
            .into_parts();
        let expected_lines = &[
            "                                        ",
            "e c h o   h i                           ",
            "                                        ",
            "                                        ",
            "— — — —   [ f i n i s h e d ]   — — — — ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "  h i                                   ",
        ];
        let expected_styles = &[
            "                                        ",
            "x x x x x x x x x x x x x x x x x x x x ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "                                        ",
            "x x x x x x x x x x x x x x x x x x x x ",
            "                                        ",
            "x x x x x x x x x x x x x x x x x x x x ",
        ];
        assert_eq!(lines, expected_lines);
        assert_eq!(styles, expected_styles);
    }

    #[test]
    fn exec_adjacent_detached_output() {
        let input = "
```bash +exec +id:foo
echo hi
```
<!-- snippet_output: foo -->";
        let lines =
            Test::new(input).render().rows(4).columns(19).run_async_renders(RunAsyncRendersPolicy::None).into_lines();
        // this should look exactly the same as if we hadn't detached the output
        let expected = &[
            //
            "                   ",
            "echo hi            ",
            "                   ",
            "—— [not started] ——",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_detached_output() {
        let input = "
```bash +exec +id:foo
echo hi
```

bar

<!-- snippet_output: foo -->";
        let lines = Test::new(input).render().rows(8).columns(16).into_lines();
        let expected = &[
            "                ",
            "echo hi         ",
            "                ",
            "—— [finished] ——",
            "                ",
            "bar             ",
            "                ",
            "hi              ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_replace() {
        let input = "
```bash +exec_replace
echo hi
```";
        let lines = Test::new(input).render().rows(3).columns(7).into_lines();
        let expected = &["       ", "hi     ", "       "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn snippet_exec_replace_centered() {
        let input = "
```bash +exec_replace
echo hi
```";
        let theme = raw::PresentationTheme {
            code: raw::CodeBlockStyle {
                alignment: Some(raw::Alignment::Center { minimum_margin: raw::Margin::Fixed(1), minimum_size: 1 }),
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(3).columns(6).into_lines();
        let expected = &["      ", "  hi  ", "      "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_replace_font_size() {
        let input = "
<!-- font_size: 2 -->
```bash +exec_replace
echo hi
```";
        let lines = Test::new(input).render().rows(3).columns(7).into_lines();
        let expected = &["       ", "h i    ", "       "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn exec_replace_long() {
        let qr = [
            "█▀▀▀▀▀█ ▄▀ ▄▀ █▀▀▀▀▀█",
            "█ ███ █ ▄▀ ▄  █ ███ █",
            "█ ▀▀▀ █ ▄▄█▀█ █ ▀▀▀ █",
            "▀▀▀▀▀▀▀ ▀ █▄█ ▀▀▀▀▀▀▀",
            "█▀▀██ ▀▀█▀  █▀ █ ▀ ▀▄",
            "▄▄██▀▄▀▀▄ █▀ ▀ ▄█▀█▀ ",
            "▀  ▀▀ ▀▀▄█▄█▄█▄▄▀ ▄ █",
            "█▀▀▀▀▀█ ▀▀ ▄█▄█▀ ▄█▀▄",
            "█ ███ █ ██▀ █  ▄█▄ ▀ ",
            "█ ▀▀▀ █ █ ▄▀ ▀  ▄██  ",
            "▀▀▀▀▀▀▀ ▀▀ ▀ ▀  ▀  ▀ ",
        ]
        .join("\n");

        let input = format!(
            r#"
```bash +exec_replace
echo "{qr}"
```
"#
        );
        let rows = 13;
        let columns = 21;
        let lines = Test::new(input).render().rows(rows).columns(columns).into_lines();
        let empty = " ".repeat(columns as usize);
        let expected: Vec<_> = [empty.as_str()].into_iter().chain(qr.lines()).chain([empty.as_str()]).collect();
        assert_eq!(lines, expected);
    }
}
