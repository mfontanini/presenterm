use crate::{
    markdown::{elements::Text, text_style::TextStyle},
    render::{
        operation::{
            AsRenderOperations, Pollable, RenderAsync, RenderAsyncStartPolicy, RenderOperation, RenderTextProperties,
            ToggleState,
        },
        properties::WindowSize,
    },
    theme::Alignment,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub(crate) struct SnippetExecutionDisabledOperation {
    text: Text,
    alignment: Alignment,
    policy: RenderAsyncStartPolicy,
    toggled: Arc<Mutex<bool>>,
}

impl SnippetExecutionDisabledOperation {
    pub(crate) fn new(
        style: TextStyle,
        alignment: Alignment,
        policy: RenderAsyncStartPolicy,
        exec_type: ExecutionType,
    ) -> Self {
        let (attribute, cli_parameter) = match exec_type {
            ExecutionType::Execute => ("+exec", "-x"),
            ExecutionType::ExecReplace => ("+exec_replace", "-X"),
            ExecutionType::Image => ("+image", "-X"),
        };
        let text = Text::new(format!("snippet {attribute} is disabled, run with {cli_parameter} to enable"), style);
        Self { text, alignment, policy, toggled: Default::default() }
    }
}

impl AsRenderOperations for SnippetExecutionDisabledOperation {
    fn as_render_operations(&self, _: &WindowSize) -> Vec<RenderOperation> {
        if !*self.toggled.lock().unwrap() {
            return Vec::new();
        }
        vec![
            RenderOperation::RenderLineBreak,
            RenderOperation::RenderText {
                line: vec![self.text.clone()].into(),
                properties: RenderTextProperties { alignment: self.alignment, ..Default::default() },
            },
            RenderOperation::RenderLineBreak,
        ]
    }
}

impl RenderAsync for SnippetExecutionDisabledOperation {
    fn pollable(&self) -> Box<dyn Pollable> {
        Box::new(ToggleState::new(self.toggled.clone()))
    }

    fn start_policy(&self) -> RenderAsyncStartPolicy {
        self.policy
    }
}

#[derive(Debug)]
pub(crate) enum ExecutionType {
    Execute,
    ExecReplace,
    Image,
}
