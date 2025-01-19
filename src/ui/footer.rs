use crate::{
    markdown::elements::Text,
    presentation::{AsRenderOperations, RenderOperation},
    render::properties::WindowSize,
    style::{Colors, TextStyle},
    theme::{Alignment, FooterStyle, Margin},
};
use std::{cell::RefCell, rc::Rc};
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
        template: &str,
        current_slide: &str,
        context: &FooterContext,
        colors: Colors,
        alignment: Alignment,
    ) -> RenderOperation {
        #[allow(clippy::literal_string_with_formatting_args)]
        let contents = template
            .replace("{current_slide}", current_slide)
            .replace("{total_slides}", &context.total_slides.to_string())
            .replace("{title}", &context.title)
            .replace("{sub_title}", &context.sub_title)
            .replace("{event}", &context.event)
            .replace("{location}", &context.location)
            .replace("{date}", &context.date)
            .replace("{author}", &context.author);
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
                vec![RenderOperation::JumpToBottomRow { index: 0 }, RenderOperation::RenderText {
                    line: vec![bar].into(),
                    alignment: Alignment::Left { margin: Margin::Fixed(0) },
                }]
            }
            FooterStyle::Empty => vec![],
        }
    }
}
