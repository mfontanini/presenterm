use crate::{
    markdown::{
        elements::{Line, Text},
        parse::{MarkdownParser, ParseInlinesError},
        text_style::{TextStyle, UndefinedPaletteColorError},
    },
    render::{
        operation::{
            AsRenderOperations, ImagePosition, ImageRenderProperties, MarginProperties, RenderOperation,
            RenderTextProperties,
        },
        properties::WindowSize,
    },
    terminal::image::Image,
    theme::{Alignment, ColorPalette, FooterContent, FooterStyle, FooterTemplate, FooterTemplateChunk, Margin},
};
use comrak::Arena;
use std::borrow::Cow;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default)]
pub(crate) struct FooterVariables {
    pub(crate) current_slide: usize,
    pub(crate) total_slides: usize,
    pub(crate) author: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) sub_title: Option<String>,
    pub(crate) event: Option<String>,
    pub(crate) location: Option<String>,
    pub(crate) date: Option<String>,
}

#[derive(Debug)]
pub(crate) struct FooterGenerator {
    current_slide: usize,
    total_slides: u64,
    style: RenderedFooterStyle,
}

impl FooterGenerator {
    pub(crate) fn new(
        style: FooterStyle,
        vars: &FooterVariables,
        palette: &ColorPalette,
    ) -> Result<Self, InvalidFooterTemplateError> {
        let style = RenderedFooterStyle::new(style, vars, palette)?;
        let current_slide = vars.current_slide;
        let total_slides = vars.total_slides as u64;
        Ok(Self { current_slide, total_slides, style })
    }

    fn render_line(line: &FooterLine, alignment: Alignment, height: u16, operations: &mut Vec<RenderOperation>) {
        operations.extend([
            RenderOperation::JumpToBottomRow { index: height / 2 },
            RenderOperation::RenderText {
                line: line.0.clone().into(),
                properties: RenderTextProperties { alignment, ..Default::default() },
            },
        ]);
    }

    fn push_image(&self, image: &Image, alignment: Alignment, operations: &mut Vec<RenderOperation>) {
        let mut properties = ImageRenderProperties::default();

        operations.push(RenderOperation::ApplyMargin(MarginProperties {
            horizontal: Margin::Fixed(0),
            top: 0,
            bottom: 1,
        }));
        match alignment {
            Alignment::Left { .. } => {
                operations.push(RenderOperation::JumpToColumn { index: 0 });
                properties.position = ImagePosition::Cursor;
            }
            Alignment::Right { .. } => {
                properties.position = ImagePosition::Right;
            }
            Alignment::Center { .. } => properties.position = ImagePosition::Center,
        };
        operations.extend([
            // Start printing the image at the top of the footer rect
            RenderOperation::JumpToRow { index: 0 },
            RenderOperation::RenderImage(image.clone(), properties),
            RenderOperation::PopMargin,
        ]);
    }
}

impl AsRenderOperations for FooterGenerator {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        use RenderedFooterStyle::*;
        match &self.style {
            Template { left, center, right, height } => {
                // Crate a margin for ourselves so we can jump to top without stepping over slide
                // text.
                let mut operations = vec![RenderOperation::ApplyMargin(MarginProperties {
                    horizontal: Margin::Fixed(1),
                    top: dimensions.rows.saturating_sub(*height),
                    bottom: 0,
                })];
                // We print this one row below the bottom so there's one row of padding.
                let alignments = [
                    Alignment::Left { margin: Default::default() },
                    Alignment::Center { minimum_size: 0, minimum_margin: Default::default() },
                    Alignment::Right { margin: Default::default() },
                ];
                for (content, alignment) in [left, center, right].iter().zip(alignments) {
                    if let Some(content) = content {
                        match content {
                            RenderedFooterContent::Line(line) => {
                                Self::render_line(line, alignment, *height, &mut operations);
                            }
                            RenderedFooterContent::Image(image) => {
                                self.push_image(image, alignment, &mut operations);
                            }
                        };
                    }
                }
                operations.push(RenderOperation::PopMargin);
                operations
            }
            ProgressBar { character, style } => {
                let character = character.to_string();
                let total_columns = dimensions.columns as usize / character.width();
                let progress_ratio = (self.current_slide + 1) as f64 / self.total_slides as f64;
                let columns_ratio = (total_columns as f64 * progress_ratio).ceil();
                let bar = character.repeat(columns_ratio as usize);
                let bar = Text::new(bar, *style);
                vec![
                    RenderOperation::JumpToBottomRow { index: 0 },
                    RenderOperation::RenderText { line: vec![bar].into(), properties: Default::default() },
                ]
            }
            Empty => vec![],
        }
    }
}

#[derive(Debug)]
enum RenderedFooterStyle {
    Template {
        left: Option<RenderedFooterContent>,
        center: Option<RenderedFooterContent>,
        right: Option<RenderedFooterContent>,
        height: u16,
    },
    ProgressBar {
        character: char,
        style: TextStyle,
    },
    Empty,
}

impl RenderedFooterStyle {
    fn new(
        style: FooterStyle,
        vars: &FooterVariables,
        palette: &ColorPalette,
    ) -> Result<Self, InvalidFooterTemplateError> {
        match style {
            FooterStyle::Template { left, center, right, style, height } => {
                let left = left.map(|c| RenderedFooterContent::new(c, &style, vars, palette)).transpose()?;
                let center = center.map(|c| RenderedFooterContent::new(c, &style, vars, palette)).transpose()?;
                let right = right.map(|c| RenderedFooterContent::new(c, &style, vars, palette)).transpose()?;
                Ok(Self::Template { left, center, right, height })
            }
            FooterStyle::ProgressBar { character, style } => Ok(Self::ProgressBar { character, style }),
            FooterStyle::Empty => Ok(Self::Empty),
        }
    }
}

#[derive(Clone, Debug)]
struct FooterLine(Line);

impl FooterLine {
    fn new(
        template: FooterTemplate,
        style: &TextStyle,
        vars: &FooterVariables,
        palette: &ColorPalette,
    ) -> Result<Self, InvalidFooterTemplateError> {
        use FooterTemplateChunk::*;
        let FooterVariables { current_slide, total_slides, author, title, sub_title, event, location, date } = vars;
        let arena = Arena::default();
        let mut reassembled = String::new();
        for chunk in template.0 {
            let raw_text = match chunk {
                CurrentSlide => Cow::Owned(current_slide.to_string()),
                OpenBrace => Cow::Borrowed("{"),
                ClosedBrace => Cow::Borrowed("}"),
                Literal(text) => Cow::Owned(text),
                TotalSlides => Cow::Owned(total_slides.to_string()),
                Author => Self::extract_variable("author", author)?,
                Title => Self::extract_variable("title", title)?,
                SubTitle => Self::extract_variable("sub_title", sub_title)?,
                Event => Self::extract_variable("event", event)?,
                Location => Self::extract_variable("location", location)?,
                Date => Self::extract_variable("date", date)?,
            };
            if raw_text.lines().count() != 1 {
                return Err(InvalidFooterTemplateError::NoNewlines);
            }
            reassembled.push_str(&raw_text);
        }
        // Inline parsing loses leading/trailing whitespaces so re-add them ourselves
        let starting_length = reassembled.len();
        let raw_text = reassembled.trim_start();
        let left_whitespace = starting_length - raw_text.len();
        let raw_text = raw_text.trim_end();
        let right_whitespace = starting_length - raw_text.len() - left_whitespace;
        let parser = MarkdownParser::new(&arena);
        let inlines = parser.parse_inlines(&reassembled)?;
        let mut line = inlines.resolve(palette)?;
        if left_whitespace != 0 {
            line.0.insert(0, " ".repeat(left_whitespace).into());
        }
        if right_whitespace != 0 {
            line.0.push(" ".repeat(right_whitespace).into());
        }
        line.apply_style(style);
        Ok(Self(line))
    }

    fn extract_variable<'a>(
        name: &'static str,
        variable: &'a Option<String>,
    ) -> Result<Cow<'a, str>, InvalidFooterTemplateError> {
        variable.as_deref().map(Cow::Borrowed).ok_or(InvalidFooterTemplateError::VariableNotSet(name))
    }
}

#[derive(Clone, Debug)]
enum RenderedFooterContent {
    Line(FooterLine),
    Image(Image),
}

impl RenderedFooterContent {
    fn new(
        content: FooterContent,
        style: &TextStyle,
        vars: &FooterVariables,
        palette: &ColorPalette,
    ) -> Result<Self, InvalidFooterTemplateError> {
        Ok(match content {
            FooterContent::Template(template) => Self::Line(FooterLine::new(template, style, vars, palette)?),
            FooterContent::Image(image) => Self::Image(image),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InvalidFooterTemplateError {
    #[error("footer cannot contain multiple lines")]
    NoNewlines,

    #[error("invalid markdown: {0}")]
    Inlines(#[from] ParseInlinesError),

    #[error(transparent)]
    PaletteColor(#[from] UndefinedPaletteColorError),

    #[error("variable '{0}' not set")]
    VariableNotSet(&'static str),
}

#[cfg(test)]
mod tests {
    use crate::markdown::text_style::Color;

    use super::*;
    use once_cell::sync::Lazy;
    use rstest::rstest;

    static VARIABLES: Lazy<FooterVariables> = Lazy::new(|| FooterVariables {
        current_slide: 1,
        total_slides: 5,
        author: Some("bob".into()),
        title: Some("hi".into()),
        sub_title: Some("bye".into()),
        event: Some("test".into()),
        location: Some("here".into()),
        date: Some("now".into()),
    });

    static PALETTE: Lazy<ColorPalette> = Lazy::new(|| ColorPalette {
        colors: [("red".into(), Color::new(255, 0, 0))].into(),
        classes: Default::default(),
    });

    #[rstest]
    #[case::literal(FooterTemplateChunk::Literal("hi".into()), &["hi".into()])]
    #[case::literal_whitespaced(FooterTemplateChunk::Literal("  hi  ".into()), &["  ".into(), "hi".into(), "  ".into()])]
    #[case::author(FooterTemplateChunk::Author, &["bob".into()])]
    #[case::title(FooterTemplateChunk::Title, &["hi".into()])]
    #[case::sub_title(FooterTemplateChunk::SubTitle, &["bye".into()])]
    #[case::event(FooterTemplateChunk::Event, &["test".into()])]
    #[case::location(FooterTemplateChunk::Location, &["here".into()])]
    #[case::date(FooterTemplateChunk::Date, &["now".into()])]
    #[case::bold(
        FooterTemplateChunk::Literal("**hi** mom".into()),
        &[Text::new("hi", TextStyle::default().bold()), " mom".into()]
    )]
    #[case::colored(
        FooterTemplateChunk::Literal("<span style=\"color: palette:red\">hi</span> mom".into()),
        &[Text::new("hi", TextStyle::default().fg_color(Color::new(255, 0, 0))), " mom".into()]
    )]
    fn render_valid(#[case] chunk: FooterTemplateChunk, #[case] expected: &[Text]) {
        let template = FooterTemplate(vec![chunk]);
        let line = FooterLine::new(template, &Default::default(), &VARIABLES, &PALETTE).expect("render failed");
        assert_eq!(line.0.0, expected);
    }

    #[rstest]
    #[case::non_paragraph(
        FooterTemplateChunk::Literal("* hi".into()),
    )]
    #[case::invalid_palette_color(
        FooterTemplateChunk::Literal("<span style=\"color: palette:hi\">hi</span> mom".into()),
    )]
    #[case::newlines(FooterTemplateChunk::Literal("hi\nmom".into()))]
    fn render_invalid(#[case] chunk: FooterTemplateChunk) {
        let template = FooterTemplate(vec![chunk]);
        FooterLine::new(template, &Default::default(), &VARIABLES, &PALETTE).expect_err("render succeeded");
    }

    #[test]
    fn interleaved_spans() {
        let chunks = vec![
            FooterTemplateChunk::Literal("<span style=\"color: palette:red\">".into()),
            FooterTemplateChunk::CurrentSlide,
            FooterTemplateChunk::Literal(" / ".into()),
            FooterTemplateChunk::TotalSlides,
            FooterTemplateChunk::Literal("</span>".into()),
            FooterTemplateChunk::Literal("<span style=\"color: green\">".into()),
            FooterTemplateChunk::Title,
            FooterTemplateChunk::Literal("</span>".into()),
        ];
        let template = FooterTemplate(chunks);
        let line = FooterLine::new(template, &Default::default(), &VARIABLES, &PALETTE).expect("render failed");
        let expected = &[
            Text::new("1 / 5", TextStyle::default().fg_color(Color::new(255, 0, 0))),
            Text::new("hi", TextStyle::default().fg_color(Color::Green)),
        ];
        assert_eq!(line.0.0, expected);
    }
}
