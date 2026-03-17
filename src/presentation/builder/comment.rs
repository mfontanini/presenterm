use crate::{
    markdown::elements::{MarkdownElement, SourcePosition},
    presentation::builder::{BuildResult, LayoutState, PresentationBuilder, error::InvalidPresentation},
    render::operation::{LayoutGrid, RenderOperation},
    theme::{Alignment, ElementType, raw},
};
use serde::Deserialize;
use std::{fmt, num::NonZeroU8, path::PathBuf, str::FromStr};
use strum::{EnumDiscriminants, EnumIter};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn process_comment(&mut self, comment: String, source_position: SourcePosition) -> BuildResult {
        let comment = comment.trim();
        let trimmed_comment = comment.trim_start_matches(&self.options.command_prefix);
        let command = match trimmed_comment.parse::<CommentCommand>() {
            Ok(comment) => comment,
            Err(error) => {
                // If we failed to parse this, make sure we shouldn't have ignored it
                if self.should_ignore_comment(comment) {
                    // Ignored comments should not add line breaks
                    self.slide_state.ignore_element_line_break = true;
                    return Ok(());
                }
                return Err(self.invalid_presentation(source_position, error));
            }
        };

        if self.options.render_speaker_notes_only {
            self.process_comment_command_speaker_notes_mode(command);
        } else {
            self.process_comment_command_presentation_mode(command, source_position)?;
        }
        Ok(())
    }

    fn process_comment_command_presentation_mode(
        &mut self,
        command: CommentCommand,
        source_position: SourcePosition,
    ) -> BuildResult {
        match command {
            CommentCommand::Pause => self.push_pause(),
            CommentCommand::BgImage(cmd) => {
                self.set_slide_background_image(cmd, source_position)?;
            }
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::NewLine => self.push_line_breaks(self.slide_font_size() as usize),
            CommentCommand::NewLines(count) => {
                self.push_line_breaks(count as usize * self.slide_font_size() as usize);
            }
            CommentCommand::Comment(_) => {}
            CommentCommand::JumpToMiddle => self.chunk_operations.push(RenderOperation::JumpToVerticalCenter),
            CommentCommand::InitColumnLayout(columns) => {
                self.validate_column_layout(&columns, source_position)?;
                let resolved_position = self.sources.resolve_source_position(source_position);
                self.slide_state.last_layout_comment = Some(resolved_position);
                self.slide_state.layout = LayoutState::InLayout { columns_count: columns.len() };
                let grid = if self.options.layout_grid {
                    LayoutGrid::Draw(self.theme.layout_grid.style)
                } else {
                    LayoutGrid::None
                };
                self.chunk_operations.push(RenderOperation::InitColumnLayout {
                    columns,
                    grid,
                    margin: self.theme.column_layout.margin,
                });
                self.slide_state.needs_enter_column = true;
            }
            CommentCommand::ResetLayout => {
                self.slide_state.layout = LayoutState::Default;
                self.chunk_operations.push(RenderOperation::ExitLayout);
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
            CommentCommand::IncrementalTables(value) => {
                self.slide_state.incremental_tables = Some(value);
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
                self.push_line_break();
            }
            CommentCommand::EndSlide => self.terminate_slide(),
            CommentCommand::Pause => self.push_pause(),
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
            comment == "{{{" || comment == "}}}" || comment.starts_with("//")
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
            self.slide_state.ignore_element_line_break = false;
            self.process_element_for_presentation_mode(element)?;
            if !self.slide_state.ignore_element_line_break {
                self.push_line_break();
            }
        }
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[serde(rename_all = "snake_case")]
pub(crate) enum CommentCommand {
    Alignment(CommentCommandAlignment),
    BgImage(BgImageCommand),
    Column(usize),
    EndSlide,
    FontSize(u8),
    Include(PathBuf),
    IncrementalLists(bool),
    IncrementalTables(bool),
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
    Comment(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct BgImageCommand {
    pub(crate) path: PathBuf,
    pub(crate) fit: Option<raw::BackgroundImageFit>,
    pub(crate) opacity: Option<u8>,
}

impl CommentCommand {
    /// Generate sample comment strings for all available commands
    pub(crate) fn generate_samples() -> Vec<&'static str> {
        use strum::IntoEnumIterator;

        CommentCommandDiscriminants::iter()
            .flat_map(|variant| {
                use CommentCommandDiscriminants::*;
                match variant {
                    Alignment => {
                        vec!["<!-- alignment: left -->", "<!-- alignment: center -->", "<!-- alignment: right -->"]
                    }
                    BgImage => vec!["<!-- bg_image: {path: bg.png} -->", "<!-- bg_image: {path: bg.png, fit: cover, opacity: 80} -->"],
                    Column => vec!["<!-- column: 0 -->"],
                    EndSlide => vec!["<!-- end_slide -->"],
                    FontSize => vec!["<!-- font_size: 2 -->"],
                    Include => vec!["<!-- include: file.md -->"],
                    IncrementalLists => {
                        vec!["<!-- incremental_lists: true -->", "<!-- incremental_lists: false -->"]
                    }
                    IncrementalTables => {
                        vec!["<!-- incremental_tables: true -->", "<!-- incremental_tables: false -->"]
                    }
                    InitColumnLayout => vec!["<!-- column_layout: [1, 2] -->"],
                    JumpToMiddle => vec!["<!-- jump_to_middle -->"],
                    ListItemNewlines => vec!["<!-- list_item_newlines: 2 -->"],
                    NewLine => vec!["<!-- new_line -->"],
                    NewLines => vec!["<!-- new_lines: 2 -->"],
                    NoFooter => vec!["<!-- no_footer -->"],
                    Pause => vec!["<!-- pause -->"],
                    ResetLayout => vec!["<!-- reset_layout -->"],
                    SkipSlide => vec!["<!-- skip_slide -->"],
                    SpeakerNote => vec!["<!-- speaker_note: Your note here -->"],
                    SnippetOutput => vec!["<!-- snippet_output: identifier -->"],
                    Comment => vec!["<!-- comment: hi mom -->"],
                }
            })
            .collect()
    }
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
pub(crate) enum CommentCommandAlignment {
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
    use crate::presentation::builder::{PresentationBuilderOptions, utils::Test};
    use image::{DynamicImage, ImageEncoder, codecs::png::PngEncoder};
    use rstest::rstest;
    use std::{fs, io::BufWriter};
    use tempfile::tempdir;

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
    #[case::comment("comment: This is a user comment", CommentCommand::Comment("This is a user comment".into()))]
    #[case::bg_image("bg_image: {path: bg.png}", CommentCommand::BgImage(BgImageCommand { path: "bg.png".into(), fit: None, opacity: None }))]
    #[case::bg_image_with_options("bg_image: {path: bg.png, fit: contain, opacity: 80}", CommentCommand::BgImage(BgImageCommand { path: "bg.png".into(), fit: Some(raw::BackgroundImageFit::Contain), opacity: Some(80) }))]
    fn command_formatting(#[case] input: &str, #[case] expected: CommentCommand) {
        let parsed: CommentCommand = input.parse().expect("deserialization failed");
        assert_eq!(parsed, expected);
    }

    #[rstest]
    #[case::multiline("hello\nworld")]
    #[case::many_open_braces("{{{")]
    #[case::many_close_braces("}}}")]
    #[case::vim_command("vim: hi")]
    #[case::padded_vim_command("vim: hi")]
    #[case::double_slash("// This is a user comment")]
    #[case::double_slash_padded("  // This is a padded comment  ")]
    #[case::comment_colon("comment: This is a user comment")]
    fn ignore_comments(#[case] comment: &str) {
        let input = format!("<!-- {comment} -->");
        Test::new(input).build();
    }

    #[rstest]
    #[case::command_with_prefix("cmd:end_slide", true)]
    #[case::non_command_with_prefix("cmd:bogus", false)]
    #[case::non_prefixed("random", true)]
    fn comment_prefix(#[case] comment: &str, #[case] should_work: bool) {
        let options = PresentationBuilderOptions { command_prefix: "cmd:".into(), ..Default::default() };

        let element = MarkdownElement::Comment { comment: comment.into(), source_position: Default::default() };
        let result = Test::new(vec![element]).options(options).try_build();
        assert_eq!(result.is_ok(), should_work, "{result:?}");
    }

    #[test]
    fn layout_without_init() {
        let input = "<!-- column: 0 -->";
        Test::new(input).expect_invalid();
    }

    #[test]
    fn already_in_column() {
        let input = "
<!-- column_layout: [1] -->
<!-- column: 0 -->
<!-- column: 0 -->
";
        Test::new(input).expect_invalid();
    }

    #[test]
    fn column_index_overflow() {
        let input = "
<!-- column_layout: [1] -->
<!-- column: 1 -->
";
        Test::new(input).expect_invalid();
    }

    #[rstest]
    #[case::empty("column_layout: []")]
    #[case::zero("column_layout: [0]")]
    #[case::one_is_zero("column_layout: [1, 0]")]
    fn invalid_layouts(#[case] definition: &str) {
        let input = format!("<!-- {definition} -->");
        Test::new(input).expect_invalid();
    }

    #[test]
    fn operation_without_enter_column() {
        let input = "
<!-- column_layout: [1] -->

# hi
";
        Test::new(input).expect_invalid();
    }

    #[test]
    fn end_slide_inside_layout() {
        let input = "
<!-- column_layout: [1] -->
<!-- end_slide -->
";
        let presentation = Test::new(input).build();
        assert_eq!(presentation.iter_slides().count(), 2);
    }

    #[test]
    fn end_slide_inside_column() {
        let input = "
<!-- column_layout: [1] -->
<!-- column: 0 -->
<!-- end_slide -->
";
        let presentation = Test::new(input).build();
        assert_eq!(presentation.iter_slides().count(), 2);
    }

    #[test]
    fn columns() {
        let input = "---
theme:
  override:
    column_layout:
      margin:
        fixed: 2
---

<!-- column_layout: [1, 1] -->
<!-- column: 0 -->
foo1

foo2

---


<!-- column: 1 -->
bar1

bar2

---
";
        let lines = Test::new(input).render().rows(7).columns(20).into_lines();
        let expected = &[
            "                    ",
            "foo1        bar1    ",
            "                    ",
            "foo2        bar2    ",
            "                    ",
            "————————    ————————",
            "                    ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn columns_back_and_forth() {
        // this is the same as the above but we run back and forth between the columns
        let input = "---
theme:
  override:
    column_layout:
      margin:
        fixed: 2
---

<!-- column_layout: [1, 1] -->

<!-- column: 0 -->
foo1

<!-- column: 1 -->

bar1


<!-- column: 0 -->

foo2

---

<!-- column: 1 -->

bar2

---
";
        let lines = Test::new(input).render().rows(7).columns(20).into_lines();
        let expected = &[
            "                    ",
            "foo1        bar1    ",
            "                    ",
            "foo2        bar2    ",
            "                    ",
            "————————    ————————",
            "                    ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn uneven_columns() {
        let input = "---
theme:
  override:
    column_layout:
      margin:
        fixed: 4
---

<!-- column_layout: [2, 1] -->
<!-- column: 0 -->
foo1

foo2

---


<!-- column: 1 -->
bar1

bar2

---
";
        let lines = Test::new(input).render().rows(7).columns(24).into_lines();
        let expected = &[
            "                        ",
            "foo1                bar1",
            "                        ",
            "foo2                bar2",
            "                        ",
            "————————————        ————",
            "                        ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn uneven_three_columns() {
        let input = "---
theme:
  override:
    column_layout:
      margin:
        fixed: 4
---

<!-- column_layout: [1, 2, 1] -->
<!-- column: 0 -->

---

<!-- column: 1 -->

---

<!-- column: 2 -->

---
";
        let lines = Test::new(input).render().rows(2).columns(32).into_lines();
        let expected = &[
            //
            "                                ",
            "————      ————————————      ————",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn pause_layout() {
        let input = r"
<!-- column_layout: [1, 1] -->
<!-- pause -->
<!-- column: 0 -->
hi
<!-- pause -->
<!-- column: 1 -->
bye
";
        let lines = Test::new(input).render().rows(5).columns(12).advances(1).into_lines();
        let expected = &["            ", "hi          ", "            ", "            ", "            "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn pause_new_slide() {
        let input = "
hi

<!-- pause -->

bye
";
        let options = PresentationBuilderOptions { pause_create_new_slide: true, ..Default::default() };
        let slides = Test::new(input).options(options).build().into_slides();
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn pause_layout_new_slide() {
        let input = r"---
theme:
  override:
    column_layout:
      margin:
        fixed: 4
---

<!-- column_layout: [1, 1] -->
<!-- column: 0 -->
hi
<!-- pause -->
<!-- column: 1 -->
bye
";
        let options = PresentationBuilderOptions { pause_create_new_slide: true, ..Default::default() };
        let lines = Test::new(input).options(options).render().rows(3).columns(15).advances(1).into_lines();
        let expected = &["               ", "hi         bye ", "               "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn skip_slide() {
        let input = "
hi

<!-- skip_slide -->
<!-- end_slide -->

bye
";
        let lines = Test::new(input).render().rows(5).columns(3).into_lines();
        let expected = &["   ", "bye", "   ", "   ", "   "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn skip_all_slides() {
        let input = "
hi

<!-- skip_slide -->
";
        let lines = Test::new(input).render().rows(5).columns(3).into_lines();
        let expected = &["   ", "   ", "   ", "   ", "   "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn skip_slide_pauses() {
        let input = "
hi

<!-- pause -->
<!-- skip_slide -->
<!-- end_slide -->

bye
";
        let lines = Test::new(input).render().rows(2).columns(3).into_lines();
        let expected = &["   ", "bye"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn skip_slide_speaker_note() {
        let input = "
hi

<!-- skip_slide -->
<!-- end_slide -->
<!-- speaker_note: bye -->
";
        let options = PresentationBuilderOptions { render_speaker_notes_only: true, ..Default::default() };
        let lines = Test::new(input).options(options).render().rows(2).columns(3).into_lines();
        let expected = &["   ", "bye"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn speaker_notes() {
        let input = "
<!-- speaker_note: hi -->

<!-- speaker_note: bye -->
";
        let options = PresentationBuilderOptions { render_speaker_notes_only: true, ..Default::default() };
        let lines = Test::new(input).options(options).render().rows(4).columns(3).into_lines();
        let expected = &["   ", "hi ", "   ", "bye"];
        assert_eq!(lines, expected);
    }

    #[test]
    fn speaker_notes_pause() {
        let input = "
<!-- speaker_note: hi -->

<!-- pause -->

<!-- speaker_note: bye -->
";
        let options = PresentationBuilderOptions { render_speaker_notes_only: true, ..Default::default() };
        let lines = Test::new(input).options(options).render().rows(4).columns(3).advances(0).into_lines();
        let expected = &["   ", "hi ", "   ", "   "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn alignment() {
        let input = "
hi

<!-- alignment: center -->

hello            

<!-- alignment: right -->

hola
";

        let lines = Test::new(input).render().rows(6).columns(16).into_lines();
        let expected = &[
            "                ",
            "hi              ",
            "                ",
            "     hello      ",
            "                ",
            "            hola",
        ];
        assert_eq!(lines, expected);
    }

    fn write_test_png(dir: &std::path::Path, name: &str) {
        let image = DynamicImage::new_rgba8(2, 2);
        let file = fs::File::create(dir.join(name)).expect("failed to create image file");
        let mut buffer = BufWriter::new(file);
        PngEncoder::new(&mut buffer)
            .write_image(image.as_bytes(), 2, 2, image.color().into())
            .expect("failed to encode png");
    }

    fn write_test_svg(dir: &std::path::Path, name: &str) {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect width="100" height="100" fill="blue"/>
        </svg>"#;
        fs::write(dir.join(name), svg).unwrap();
    }

    #[test]
    fn bg_image_comment_loads_png() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_png(dir.path(), "bg.png");

        let input = "<!-- bg_image: {path: bg.png, fit: stretch} -->\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn bg_image_comment_loads_svg() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_svg(dir.path(), "bg.svg");

        let input = "<!-- bg_image: {path: bg.svg, fit: stretch} -->\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn bg_image_comment_cover_mode() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_png(dir.path(), "bg.png");

        let input = "<!-- bg_image: {path: bg.png, fit: cover} -->\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn bg_image_comment_with_opacity() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_png(dir.path(), "bg.png");

        let input = "<!-- bg_image: {path: bg.png, opacity: 50} -->\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn bg_image_comment_svg_cover() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_svg(dir.path(), "bg.svg");

        let input = "<!-- bg_image: {path: bg.svg, fit: cover} -->\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn bg_image_comment_missing_file() {
        let dir = tempdir().expect("failed to create tempdir");

        let input = "<!-- bg_image: {path: missing.png} -->\nhello";
        Test::new(input).resources_path(dir.path()).expect_invalid();
    }

    #[test]
    fn theme_background_image_stretch() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_png(dir.path(), "bg.png");

        let input = "---\ntheme:\n  override:\n    default:\n      background_image:\n        path: bg.png\n        fit: stretch\n---\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn theme_background_image_svg() {
        let dir = tempdir().expect("failed to create tempdir");
        write_test_svg(dir.path(), "bg.svg");

        let input = "---\ntheme:\n  override:\n    default:\n      background_image:\n        path: bg.svg\n        fit: stretch\n---\nhello";
        Test::new(input).resources_path(dir.path()).build();
    }

    #[test]
    fn include() {
        let dir = tempdir().expect("failed to created tempdir");
        let path = dir.path();
        let inner_path = path.join("inner");
        fs::create_dir_all(path.join(&inner_path)).expect("failed to create dir");

        let image = DynamicImage::new_rgba8(1, 1);
        let mut buffer = BufWriter::new(fs::File::create(inner_path.join("img.png")).expect("failed to write image"));
        PngEncoder::new(&mut buffer)
            .write_image(image.as_bytes(), 1, 1, image.color().into())
            .expect("failed to create imager");
        drop(buffer);

        fs::write(
            path.join("first.md"),
            r"
first
===

foo

![](inner/img.png)

<!-- include: inner/second.md -->

```file
path: inner/foo.txt
language: text
```
",
        )
        .unwrap();

        fs::write(
            inner_path.join("second.md"),
            r"
<!-- column_layout: [1] -->
<!-- column: 0 -->
second
<!-- reset_layout -->

![](img.png)
",
        )
        .unwrap();

        fs::write(inner_path.join("foo.txt"), "a").unwrap();

        let input = "
hi

<!-- include: first.md -->
        ";

        let lines = Test::new(input).resources_path(path).render().rows(14).columns(12).into_lines();
        let expected = &[
            "            ",
            "hi          ",
            "            ",
            "first       ",
            "            ",
            "foo         ",
            "            ",
            "            ",
            "            ",
            "second      ",
            "            ",
            "            ",
            "            ",
            "a           ",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn self_include() {
        let dir = tempdir().expect("failed to created tempdir");
        let path = dir.path();

        fs::write(path.join("main.md"), "<!-- include: main.md -->").unwrap();
        let input = "<!-- include: main.md -->";

        let err = Test::new(input).resources_path(path).expect_invalid();
        assert!(err.to_string().contains("was already imported"), "{err:?}");
    }

    #[test]
    fn include_cycle() {
        let dir = tempdir().expect("failed to created tempdir");
        let path = dir.path();

        fs::write(path.join("main.md"), "<!-- include: inner.md -->").unwrap();
        fs::write(path.join("inner.md"), "<!-- include: main.md -->").unwrap();
        let input = "<!-- include: main.md -->";

        let err = Test::new(input).resources_path(path).expect_invalid();
        assert!(err.to_string().contains("was already imported"), "{err:?}");
    }
}
