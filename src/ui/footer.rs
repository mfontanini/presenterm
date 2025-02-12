use crate::{
    markdown::{
        elements::Text,
        text_style::{Colors, TextStyle},
    },
    render::{
        operation::{AsRenderOperations, ImageRenderProperties, MarginProperties, RenderOperation},
        properties::WindowSize,
    },
    resource::Resources,
    terminal::image::printer::RegisterImageError,
    theme::{Alignment, FooterContent, FooterStyle, FooterTemplate, FooterTemplateChunk, Margin},
};
use std::{
    cell::RefCell,
    io::{BufWriter, Write},
    path::Path,
    rc::Rc,
};
use unicode_width::UnicodeWidthStr;

pub(crate) const DEFAULT_FOOTER_HEIGHT: u16 = 3;

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
    current_slide: usize,
    context: Rc<RefCell<FooterContext>>,
    style: FooterStyle,
    resources: Resources,
}

impl FooterGenerator {
    pub(crate) fn new(
        current_slide: usize,
        context: Rc<RefCell<FooterContext>>,
        style: FooterStyle,
        resources: Resources,
    ) -> Result<Self, RegisterImageError> {
        if let FooterStyle::Template { left, center, .. } = &style {
            for content in [left, center].into_iter().flatten() {
                // Load it to make sure we can
                if let FooterContent::Image { path } = content {
                    resources.theme_image(path)?;
                }
            }
        }
        Ok(Self { current_slide, context, style, resources })
    }

    fn render_template(
        template: &FooterTemplate,
        current_slide: &str,
        context: &FooterContext,
        colors: Colors,
        alignment: Alignment,
        operations: &mut Vec<RenderOperation>,
    ) {
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
        operations.extend([
            RenderOperation::JumpToBottomRow { index: 1 },
            RenderOperation::RenderText { line: vec![text].into(), alignment },
        ]);
    }

    fn push_image(
        &self,
        path: &Path,
        alignment: Alignment,
        dimensions: &WindowSize,
        operations: &mut Vec<RenderOperation>,
    ) {
        let image = self.resources.theme_image(path).expect("footer images should be loaded by now");
        let mut properties = ImageRenderProperties { center: false, ..Default::default() };

        operations.push(RenderOperation::ApplyMargin(MarginProperties {
            horizontal: Margin::Fixed(0),
            top: 0,
            bottom: 1,
        }));
        match alignment {
            Alignment::Left { .. } => {
                operations.push(RenderOperation::JumpToColumn { index: 0 });
            }
            Alignment::Right { .. } => {
                operations.push(RenderOperation::JumpToColumn { index: dimensions.columns.saturating_sub(1) });
            }
            Alignment::Center { .. } => properties.center = true,
        };
        operations.extend([
            // Start printing the image at the top of the footer rect
            RenderOperation::JumpToRow { index: 0 },
            RenderOperation::RenderImage(image, properties),
            RenderOperation::PopMargin,
        ]);
    }
}

impl AsRenderOperations for FooterGenerator {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let context = self.context.borrow();
        match &self.style {
            FooterStyle::Template { left, center, right, colors, height } => {
                let current_slide = (self.current_slide + 1).to_string();
                // Crate a margin for ourselves so we can jump to top without stepping over slide
                // text.
                let mut operations = vec![RenderOperation::ApplyMargin(MarginProperties {
                    horizontal: Margin::Fixed(1),
                    top: dimensions.rows.saturating_sub(height.unwrap_or(DEFAULT_FOOTER_HEIGHT)),
                    bottom: 0,
                })];
                // We print this one row below the bottom so there's one row of padding.
                let alignments = [
                    Alignment::Left { margin: Default::default() },
                    Alignment::Center { minimum_size: 0, minimum_margin: Default::default() },
                ];
                for (content, alignment) in [left, center].iter().zip(alignments) {
                    if let Some(content) = content {
                        match content {
                            FooterContent::Template(template) => {
                                Self::render_template(
                                    template,
                                    &current_slide,
                                    &context,
                                    *colors,
                                    alignment,
                                    &mut operations,
                                );
                            }
                            FooterContent::Image { path } => {
                                self.push_image(path, alignment, dimensions, &mut operations);
                            }
                        };
                    }
                }
                // We don't support images on the right so treat this differently
                if let Some(template) = right {
                    Self::render_template(
                        template,
                        &current_slide,
                        &context,
                        *colors,
                        Alignment::Right { margin: Default::default() },
                        &mut operations,
                    );
                }
                operations.push(RenderOperation::PopMargin);
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
