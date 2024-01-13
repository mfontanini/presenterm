use crate::{
    presentation::{AsRenderOperations, RenderOperation},
    render::properties::WindowSize,
};
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub(crate) struct RenderSeparator {
    heading: String,
}

impl RenderSeparator {
    pub(crate) fn new<S: Into<String>>(heading: S) -> Self {
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
        let separator = match self.heading.is_empty() {
            true => character.repeat(dimensions.columns as usize),
            false => {
                let dashes_len = (dimensions.columns as usize).saturating_sub(self.heading.len()) / 2;
                let dashes = character.repeat(dashes_len);
                let heading = &self.heading;
                format!("{dashes}{heading}{dashes}")
            }
        };
        vec![RenderOperation::RenderText { line: separator.into(), alignment: Default::default() }]
    }

    fn diffable_content(&self) -> Option<&str> {
        None
    }
}
