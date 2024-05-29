use crate::{
    markdown::elements::TextBlock,
    presentation::{AsRenderOperations, RenderOperation},
    render::properties::WindowSize,
};
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub(crate) struct RenderSeparator {
    heading: TextBlock,
}

impl RenderSeparator {
    pub(crate) fn new<S: Into<TextBlock>>(heading: S) -> Self {
        Self { heading: heading.into() }
    }
}

impl From<RenderSeparator> for RenderOperation {
    fn from(separator: RenderSeparator) -> Self {
        Self::RenderDynamic(Rc::new(separator))
    }
}

impl AsRenderOperations for RenderSeparator {
    fn as_render_operations(&self, dimensions: &WindowSize) -> Vec<RenderOperation> {
        let character = "—";
        let separator = match self.heading.width() == 0 {
            true => TextBlock::from(character.repeat(dimensions.columns as usize)),
            false => {
                let dashes_len = (dimensions.columns as usize).saturating_sub(self.heading.width()) / 2;
                let dashes = character.repeat(dashes_len);
                let mut line = TextBlock::from(dashes.clone());
                line.0.extend(self.heading.0.iter().cloned());
                line.0.push(dashes.into());
                line
            }
        };
        vec![RenderOperation::RenderText { line: separator.into(), alignment: Default::default() }]
    }

    fn diffable_content(&self) -> Option<&str> {
        None
    }
}
