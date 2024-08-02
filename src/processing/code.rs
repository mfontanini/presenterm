use super::padding::NumberPadder;
use crate::{
    markdown::{
        elements::{HighlightGroup, Snippet},
        text::{WeightedText, WeightedTextBlock},
    },
    presentation::{AsRenderOperations, BlockLine, ChunkMutator, RenderOperation},
    render::{
        highlighting::{LanguageHighlighter, StyledTokens},
        properties::WindowSize,
    },
    style::{Color, TextStyle},
    theme::{Alignment, CodeBlockStyle},
    PresentationTheme,
};
use std::{cell::RefCell, rc::Rc};
use unicode_width::UnicodeWidthStr;

pub(crate) struct CodePreparer<'a> {
    theme: &'a PresentationTheme,
}

impl<'a> CodePreparer<'a> {
    pub(crate) fn new(theme: &'a PresentationTheme) -> Self {
        Self { theme }
    }

    pub(crate) fn prepare(&self, code: &Snippet) -> Vec<CodeLine> {
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

    fn push_lines(&self, code: &Snippet, horizontal_padding: u8, lines: &mut Vec<CodeLine>) {
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
        dim_style: &TextStyle,
        code_highlighter: &mut LanguageHighlighter,
        block_style: &CodeBlockStyle,
    ) -> WeightedTextBlock {
        let mut output = code_highlighter.highlight_line(&self.code, block_style).0;
        output.push(StyledTokens { style: *dim_style, tokens: &self.suffix }.apply_style());
        output.into()
    }

    pub(crate) fn dim(&self, dim_style: &TextStyle) -> WeightedTextBlock {
        let mut output = Vec::new();
        for chunk in [&self.code, &self.suffix] {
            output.push(StyledTokens { style: *dim_style, tokens: chunk }.apply_style());
        }
        output.into()
    }

    pub(crate) fn dim_prefix(&self, dim_style: &TextStyle) -> WeightedText {
        let text = StyledTokens { style: *dim_style, tokens: &self.prefix }.apply_style();
        text.into()
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
    pub(crate) prefix: WeightedText,
    pub(crate) highlighted: WeightedTextBlock,
    pub(crate) not_highlighted: WeightedTextBlock,
    pub(crate) line_number: Option<u16>,
    pub(crate) context: Rc<RefCell<HighlightContext>>,
    pub(crate) block_color: Option<Color>,
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
            RenderOperation::RenderBlockLine(BlockLine {
                prefix: self.prefix.clone(),
                text,
                block_length: context.block_length as u16,
                alignment: context.alignment.clone(),
                block_color: self.block_color,
            }),
            RenderOperation::RenderLineBreak,
        ]
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
    use crate::markdown::elements::{SnippetAttributes, SnippetLanguage};

    use super::*;

    #[test]
    fn code_with_line_numbers() {
        let total_lines = 11;
        let input_lines = "hi\n".repeat(total_lines);
        let code = Snippet {
            contents: input_lines,
            language: SnippetLanguage::Unknown("".to_string()),
            attributes: SnippetAttributes { line_numbers: true, ..Default::default() },
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
