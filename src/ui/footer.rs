use crate::{
    markdown::{
        elements::Text,
        text_style::{Colors, TextStyle},
    },
    render::{
        operation::{AsRenderOperations, RenderOperation},
        properties::WindowSize,
    },
    theme::{Alignment, FooterStyle, FooterTemplate, FooterTemplateChunk, Margin},
};
use std::{
    cell::RefCell,
    io::{BufWriter, Write},
    rc::Rc,
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default)]
pub(crate) struct FooterContext {
    pub(crate) total_slides: usize,
    pub(crate) author: String,
    pub(crate) title: String,
    pub(crate) sub_title: String,
    pub(crate) event: String,
    pub(crate) location: String,
    pub(crate) date: String,
}

#[derive(Debug)]
pub(crate) struct FooterGenerator {
    pub(crate) current_slide: usize,
    pub(crate) context: Rc<RefCell<FooterContext>>,
    pub(crate) style: FooterStyle,
}

impl FooterGenerator {
    fn render_template(
        template: &FooterTemplate,
        current_slide: &str,
        context: &FooterContext,
        colors: Colors,
        alignment: Alignment,
    ) -> RenderOperation {
        use FooterTemplateChunk::*;
        let mut w = BufWriter::new(Vec::new());
        let FooterContext { total_slides, author, title, sub_title, event, location, date } = context;
        for chunk in &template.0 {
            match chunk {
                Literal(l) => write!(w, "{l}"),
                CurrentSlide => write!(w, "{current_slide}"),
                TotalSlides => write!(w, "{total_slides}"),
                Author => write!(w, "{author}"),
                Title => write!(w, "{title}"),
                SubTitle => write!(w, "{sub_title}"),
                Event => write!(w, "{event}"),
                Location => write!(w, "{location}"),
                Date => write!(w, "{date}"),
            }
            .unwrap();
        }
        let contents = String::from_utf8(w.into_inner().unwrap()).expect("not utf8");
        let text = Text::new(contents, TextStyle::default().colors(colors));
        RenderOperation::RenderText { line: vec![text].into(), alignment }
    }
}

impl AsRenderOperations for FooterGenerator {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let context = self.context.borrow();
        match &self.style {
            FooterStyle::Template { left, center, right, colors } => {
                let current_slide = (self.current_slide + 1).to_string();
                // We print this one row below the bottom so there's one row of padding.
                let mut operations = vec![RenderOperation::JumpToBottomRow { index: 1 }];
                let margin = Margin::Fixed(1);
                let alignments = [
                    Alignment::Left { margin: margin.clone() },
                    Alignment::Center { minimum_size: 0, minimum_margin: margin.clone() },
                    Alignment::Right { margin: margin.clone() },
                ];
                for (text, alignment) in [left, center, right].iter().zip(alignments) {
                    if let Some(text) = text {
                        operations.push(Self::render_template(text, &current_slide, &context, *colors, alignment));
                    }
                }
                operations
            }
            FooterStyle::ProgressBar { character, colors } => {
                let character = character.unwrap_or('█').to_string();
                let total_columns = dimensions.columns as usize / character.width();
                let progress_ratio = (self.current_slide + 1) as f64 / context.total_slides as f64;
                let columns_ratio = (total_columns as f64 * progress_ratio).ceil();
                let bar = character.repeat(columns_ratio as usize);
                let bar = Text::new(bar, TextStyle::default().colors(*colors));
                vec![
                    RenderOperation::JumpToBottomRow { index: 0 },
                    RenderOperation::RenderText {
                        line: vec![bar].into(),
                        alignment: Alignment::Left { margin: Margin::Fixed(0) },
                    },
                ]
            }
            FooterStyle::Empty => vec![],
        }
    }
}
