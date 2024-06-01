use super::padding::NumberPadder;
use crate::{
    markdown::elements::{Code, HighlightGroup},
    presentation::{AsRenderOperations, ChunkMutator, PreformattedLine, RenderOperation},
    render::{
        highlighting::{LanguageHighlighter, StyledTokens},
        properties::WindowSize,
    },
    theme::{Alignment, CodeBlockStyle},
    PresentationTheme,
};
use std::{cell::RefCell, rc::Rc};
use syntect::highlighting::Style;
use unicode_width::UnicodeWidthStr;

pub(crate) struct CodePreparer<'a> {
    theme: &'a PresentationTheme,
}

impl<'a> CodePreparer<'a> {
    pub(crate) fn new(theme: &'a PresentationTheme) -> Self {
        Self { theme }
    }

    pub(crate) fn prepare(&self, code: &Code) -> Vec<CodeLine> {
        let mut lines = Vec::new();
        let horizontal_padding = self.theme.code.padding.horizontal.unwrap_or(0);
        let vertical_padding = self.theme.code.padding.vertical.unwrap_or(0);
        if vertical_padding > 0 {
            lines.push(CodeLine::empty());
        }
        self.push_lines(code, horizontal_padding, &mut lines);
        if vertical_padding > 0 {
            lines.push(CodeLine::empty());
        }
        lines
    }

    fn push_lines(&self, code: &Code, horizontal_padding: u8, lines: &mut Vec<CodeLine>) {
        if code.contents.is_empty() {
            return;
        }

        let padding = " ".repeat(horizontal_padding as usize);
        let padder = NumberPadder::new(code.visible_lines().count());
        for (index, line) in code.visible_lines().enumerate() {
            let mut line = line.to_string();
            let mut prefix = padding.clone();
            if code.attributes.line_numbers {
                let line_number = index + 1;
                prefix.push_str(&padder.pad_right(line_number));
                prefix.push(' ');
            }
            line.push('\n');
            let line_number = Some(index as u16 + 1);
            lines.push(CodeLine { prefix, code: line, suffix: padding.clone(), line_number });
        }
    }
}

pub(crate) struct CodeLine {
    pub(crate) prefix: String,
    pub(crate) code: String,
    pub(crate) suffix: String,
    pub(crate) line_number: Option<u16>,
}

impl CodeLine {
    pub(crate) fn empty() -> Self {
        Self { prefix: String::new(), code: "\n".into(), suffix: String::new(), line_number: None }
    }

    pub(crate) fn width(&self) -> usize {
        self.prefix.width() + self.code.width() + self.suffix.width()
    }

    pub(crate) fn highlight(
        &self,
        padding_style: &Style,
        code_highlighter: &mut LanguageHighlighter,
        block_style: &CodeBlockStyle,
    ) -> String {
        let mut output = StyledTokens { style: *padding_style, tokens: &self.prefix }.apply_style(block_style);
        output.push_str(&code_highlighter.highlight_line(&self.code, block_style));
        output.push_str(&StyledTokens { style: *padding_style, tokens: &self.suffix }.apply_style(block_style));
        output
    }
}

#[derive(Debug)]
pub(crate) struct HighlightContext {
    pub(crate) groups: Vec<HighlightGroup>,
    pub(crate) current: usize,
    pub(crate) block_length: usize,
    pub(crate) alignment: Alignment,
}

#[derive(Debug)]
pub(crate) struct HighlightedLine {
    pub(crate) highlighted: String,
    pub(crate) not_highlighted: String,
    pub(crate) line_number: Option<u16>,
    pub(crate) width: usize,
    pub(crate) context: Rc<RefCell<HighlightContext>>,
}

impl AsRenderOperations for HighlightedLine {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        let context = self.context.borrow();
        let group = &context.groups[context.current];
        let needs_highlight = self.line_number.map(|number| group.contains(number)).unwrap_or_default();
        // TODO: Cow<str>?
        let text = match needs_highlight {
            true => self.highlighted.clone(),
            false => self.not_highlighted.clone(),
        };
        vec![
            RenderOperation::RenderPreformattedLine(PreformattedLine {
                text,
                unformatted_length: self.width as u16,
                block_length: context.block_length as u16,
                alignment: context.alignment.clone(),
            }),
            RenderOperation::RenderLineBreak,
        ]
    }

    fn diffable_content(&self) -> Option<&str> {
        Some(&self.highlighted)
    }
}

#[derive(Debug)]
pub(crate) struct HighlightMutator {
    context: Rc<RefCell<HighlightContext>>,
}

impl HighlightMutator {
    pub(crate) fn new(context: Rc<RefCell<HighlightContext>>) -> Self {
        Self { context }
    }
}

impl ChunkMutator for HighlightMutator {
    fn mutate_next(&self) -> bool {
        let mut context = self.context.borrow_mut();
        if context.current == context.groups.len() - 1 {
            false
        } else {
            context.current += 1;
            true
        }
    }

    fn mutate_previous(&self) -> bool {
        let mut context = self.context.borrow_mut();
        if context.current == 0 {
            false
        } else {
            context.current -= 1;
            true
        }
    }

    fn reset_mutations(&self) {
        self.context.borrow_mut().current = 0;
    }

    fn apply_all_mutations(&self) {
        let mut context = self.context.borrow_mut();
        context.current = context.groups.len() - 1;
    }

    fn mutations(&self) -> (usize, usize) {
        let context = self.context.borrow();
        (context.current, context.groups.len())
    }
}

#[cfg(test)]
mod test {
    use crate::markdown::elements::{CodeAttributes, CodeLanguage};

    use super::*;

    #[test]
    fn code_with_line_numbers() {
        let total_lines = 11;
        let input_lines = "hi\n".repeat(total_lines);
        let code = Code {
            contents: input_lines,
            language: CodeLanguage::Unknown,
            attributes: CodeAttributes { line_numbers: true, ..Default::default() },
        };
        let lines = CodePreparer { theme: &Default::default() }.prepare(&code);
        assert_eq!(lines.len(), total_lines);

        let mut lines = lines.into_iter().enumerate();
        // 0..=9
        for (index, line) in lines.by_ref().take(9) {
            let line_number = index + 1;
            assert_eq!(&line.prefix, &format!(" {line_number} "));
        }
        // 10..
        for (index, line) in lines {
            let line_number = index + 1;
            assert_eq!(&line.prefix, &format!("{line_number} "));
        }
    }
}
