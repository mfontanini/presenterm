use std::{fmt, num::NonZeroU8, path::PathBuf, str::FromStr};

use crate::{
    markdown::elements::{MarkdownElement, SourcePosition},
    presentation::builder::{BuildResult, LayoutState, PresentationBuilder, error::InvalidPresentation},
    render::operation::RenderOperation,
    theme::{Alignment, ElementType},
};
use serde::Deserialize;

impl PresentationBuilder<'_, '_> {
    pub(crate) fn process_comment(&mut self, comment: String, source_position: SourcePosition) -> BuildResult {
        let comment = comment.trim();
        let trimmed_comment = comment.trim_start_matches(&self.options.command_prefix);
        let command = match trimmed_comment.parse::<CommentCommand>() {
            Ok(comment) => comment,
            Err(error) => {
                // If we failed to parse this, make sure we shouldn't have ignored it
                if self.should_ignore_comment(comment) {
                    return Ok(());
                }
                return Err(self.invalid_presentation(source_position, error));
            }
        };

        if self.options.render_speaker_notes_only {
            self.process_comment_command_speaker_notes_mode(command);
            Ok(())
        } else {
            self.process_comment_command_presentation_mode(command, source_position)
        }
    }

    fn process_comment_command_presentation_mode(
        &mut self,
        command: CommentCommand,
        source_position: SourcePosition,
    ) -> BuildResult {
        match command {
            CommentCommand::Pause => self.push_pause(),
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::NewLine => self.push_line_breaks(self.slide_font_size() as usize),
            CommentCommand::NewLines(count) => {
                self.push_line_breaks(count as usize * self.slide_font_size() as usize);
            }
            CommentCommand::JumpToMiddle => self.chunk_operations.push(RenderOperation::JumpToVerticalCenter),
            CommentCommand::InitColumnLayout(columns) => {
                self.validate_column_layout(&columns, source_position)?;
                self.slide_state.layout = LayoutState::InLayout { columns_count: columns.len() };
                self.chunk_operations.push(RenderOperation::InitColumnLayout { columns });
                self.slide_state.needs_enter_column = true;
            }
            CommentCommand::ResetLayout => {
                self.slide_state.layout = LayoutState::Default;
                self.chunk_operations.extend([RenderOperation::ExitLayout, RenderOperation::RenderLineBreak]);
            }
            CommentCommand::Column(column) => {
                let (current_column, columns_count) = match self.slide_state.layout {
                    LayoutState::InColumn { column, columns_count } => (Some(column), columns_count),
                    LayoutState::InLayout { columns_count } => (None, columns_count),
                    LayoutState::Default => {
                        return Err(self.invalid_presentation(source_position, InvalidPresentation::NoLayout));
                    }
                };
                if current_column == Some(column) {
                    return Err(self.invalid_presentation(source_position, InvalidPresentation::AlreadyInColumn));
                } else if column >= columns_count {
                    return Err(self.invalid_presentation(source_position, InvalidPresentation::ColumnIndexTooLarge));
                }
                self.slide_state.layout = LayoutState::InColumn { column, columns_count };
                self.chunk_operations.push(RenderOperation::EnterColumn { column });
            }
            CommentCommand::IncrementalLists(value) => {
                self.slide_state.incremental_lists = Some(value);
            }
            CommentCommand::NoFooter => {
                self.slide_state.ignore_footer = true;
            }
            CommentCommand::SpeakerNote(_) => {}
            CommentCommand::FontSize(size) => {
                if size == 0 || size > 7 {
                    return Err(self.invalid_presentation(source_position, InvalidPresentation::InvalidFontSize));
                }
                self.slide_state.font_size = Some(size)
            }
            CommentCommand::Alignment(alignment) => {
                let alignment = match alignment {
                    CommentCommandAlignment::Left => Alignment::Left { margin: Default::default() },
                    CommentCommandAlignment::Center => {
                        Alignment::Center { minimum_margin: Default::default(), minimum_size: Default::default() }
                    }
                    CommentCommandAlignment::Right => Alignment::Right { margin: Default::default() },
                };
                self.slide_state.alignment = Some(alignment);
            }
            CommentCommand::SkipSlide => {
                self.slide_state.skip_slide = true;
            }
            CommentCommand::ListItemNewlines(count) => {
                self.slide_state.list_item_newlines = Some(count.into());
            }
            CommentCommand::Include(path) => {
                self.process_include(path, source_position)?;
                return Ok(());
            }
            CommentCommand::SnippetOutput(id) => {
                let handle = self.executable_snippets.get(&id).cloned().ok_or_else(|| {
                    self.invalid_presentation(source_position, InvalidPresentation::UndefinedSnippetId(id))
                })?;
                self.push_detached_code_execution(handle)?;
                return Ok(());
            }
        };
        // Don't push line breaks for any comments.
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    fn process_comment_command_speaker_notes_mode(&mut self, comment_command: CommentCommand) {
        match comment_command {
            CommentCommand::SpeakerNote(note) => {
                for line in note.lines() {
                    self.push_text(line.into(), ElementType::Paragraph);
                    self.push_line_break();
                }
            }
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::SkipSlide => self.slide_state.skip_slide = true,
            _ => {}
        }
    }

    fn should_ignore_comment(&self, comment: &str) -> bool {
        if comment.contains('\n') || !comment.starts_with(&self.options.command_prefix) {
            // Ignore any multi line comment; those are assumed to be user comments
            // Ignore any line that doesn't start with the selected prefix.
            true
        } else if comment.trim().starts_with("vim:") {
            // ignore vim: commands
            true
        } else {
            // Ignore vim-like code folding tags
            let comment = comment.trim();
            comment == "{{{" || comment == "}}}"
        }
    }

    fn process_include(&mut self, path: PathBuf, source_position: SourcePosition) -> BuildResult {
        let base = self.resource_base_path();
        let resolved_path = self.resources.resolve_path(&path, &base);
        let contents = self.resources.external_text_file(&path, &base).map_err(|e| {
            self.invalid_presentation(
                source_position,
                InvalidPresentation::IncludeMarkdown { path: path.clone(), error: e },
            )
        })?;
        let elements = self.markdown_parser.parse(&contents).map_err(|e| {
            self.invalid_presentation(
                source_position,
                InvalidPresentation::ParseInclude { path: path.clone(), error: e },
            )
        })?;
        let _guard = self
            .sources
            .enter(resolved_path)
            .map_err(|e| self.invalid_presentation(source_position, InvalidPresentation::Import { path, error: e }))?;
        for element in elements {
            if let MarkdownElement::FrontMatter(_) = element {
                return Err(self.invalid_presentation(source_position, InvalidPresentation::IncludeFrontMatter));
            }
            self.process_element_for_presentation_mode(element)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CommentCommand {
    Alignment(CommentCommandAlignment),
    Column(usize),
    EndSlide,
    FontSize(u8),
    Include(PathBuf),
    IncrementalLists(bool),
    #[serde(rename = "column_layout")]
    InitColumnLayout(Vec<u8>),
    JumpToMiddle,
    ListItemNewlines(NonZeroU8),
    #[serde(alias = "newline")]
    NewLine,
    #[serde(alias = "newlines")]
    NewLines(u32),
    NoFooter,
    Pause,
    ResetLayout,
    SkipSlide,
    SpeakerNote(String),
    SnippetOutput(String),
}

impl FromStr for CommentCommand {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Deserialize)]
        struct CommandWrapper(#[serde(with = "serde_yaml::with::singleton_map")] CommentCommand);

        let wrapper = serde_yaml::from_str::<CommandWrapper>(s)?;
        Ok(wrapper.0)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CommentCommandAlignment {
    Left,
    Center,
    Right,
}

#[derive(thiserror::Error, Debug)]
pub struct CommandParseError(#[from] serde_yaml::Error);

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.0.to_string();
        // Remove the trailing "at line X, ..." that comes from serde_yaml. This otherwise claims
        // we're always in line 1 because the yaml is parsed in isolation out of the HTML comment.
        let inner = inner.split(" at line").next().unwrap();
        write!(f, "{inner}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::pause("pause", CommentCommand::Pause)]
    #[case::pause(" pause ", CommentCommand::Pause)]
    #[case::end_slide("end_slide", CommentCommand::EndSlide)]
    #[case::column_layout("column_layout: [1, 2]", CommentCommand::InitColumnLayout(vec![1, 2]))]
    #[case::column("column: 1", CommentCommand::Column(1))]
    #[case::reset_layout("reset_layout", CommentCommand::ResetLayout)]
    #[case::incremental_lists("incremental_lists: true", CommentCommand::IncrementalLists(true))]
    #[case::incremental_lists("new_lines: 2", CommentCommand::NewLines(2))]
    #[case::incremental_lists("newlines: 2", CommentCommand::NewLines(2))]
    #[case::incremental_lists("new_line", CommentCommand::NewLine)]
    #[case::incremental_lists("newline", CommentCommand::NewLine)]
    fn command_formatting(#[case] input: &str, #[case] expected: CommentCommand) {
        let parsed: CommentCommand = input.parse().expect("deserialization failed");
        assert_eq!(parsed, expected);
    }
}
