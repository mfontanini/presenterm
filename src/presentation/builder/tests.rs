use super::*;
use crate::{
    markdown::elements::{Table, TableRow},
    presentation::{Slide, builder::sources::MarkdownSourceError},
};
use image::{ImageEncoder, codecs::png::PngEncoder};
use rstest::rstest;
use std::{fs, io::BufWriter, path::PathBuf};
use tempfile::tempdir;

struct MemoryPresentationReader {
    contents: String,
}

impl PresentationReader for MemoryPresentationReader {
    fn read(&self, _path: &Path) -> io::Result<String> {
        Ok(self.contents.clone())
    }
}

pub(crate) enum Input {
    Markdown(String),
    Parsed(Vec<MarkdownElement>),
}

impl From<&'_ str> for Input {
    fn from(value: &'_ str) -> Self {
        Self::Markdown(value.to_string())
    }
}

impl From<String> for Input {
    fn from(value: String) -> Self {
        Self::Markdown(value)
    }
}

impl From<Vec<MarkdownElement>> for Input {
    fn from(value: Vec<MarkdownElement>) -> Self {
        Self::Parsed(value)
    }
}

pub(crate) struct Test {
    input: Input,
    options: PresentationBuilderOptions,
    resources_path: PathBuf,
}

impl Test {
    pub(crate) fn new<T: Into<Input>>(input: T) -> Self {
        Self { input: input.into(), options: Default::default(), resources_path: std::env::temp_dir() }
    }

    pub(crate) fn options(mut self, options: PresentationBuilderOptions) -> Self {
        self.options = options;
        self
    }

    pub(crate) fn resources_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.resources_path = path.into();
        self
    }

    pub(crate) fn with_builder<T, F>(&self, callback: F) -> T
    where
        F: for<'a, 'b> Fn(PresentationBuilder<'a, 'b>) -> T,
    {
        let theme = raw::PresentationTheme::default();
        let resources = Resources::new(&self.resources_path, &self.resources_path, Default::default());
        let mut third_party = ThirdPartyRender::default();
        let code_executor = Arc::new(SnippetExecutor::default());
        let themes = Themes::default();
        let bindings = KeyBindingsConfig::default();
        let arena = Default::default();
        let parser = MarkdownParser::new(&arena);
        let builder = PresentationBuilder::new(
            &theme,
            resources,
            &mut third_party,
            code_executor,
            &themes,
            Default::default(),
            bindings,
            &parser,
            self.options.clone(),
        )
        .expect("failed to create builder");
        callback(builder)
    }

    pub(crate) fn build(self) -> Presentation {
        self.try_build().expect("build failed")
    }

    pub(crate) fn expect_invalid(self) -> BuildError {
        self.try_build().expect_err("build succeeded")
    }

    fn try_build(self) -> Result<Presentation, BuildError> {
        self.with_builder(|builder| match &self.input {
            Input::Markdown(input) => {
                let reader = MemoryPresentationReader { contents: input.clone() };
                let path = self.resources_path.join("presentation.md");
                builder.build_with_reader(&path, reader)
            }
            Input::Parsed(elements) => builder.build_from_parsed(elements.clone()),
        })
    }
}

fn is_visible(operation: &RenderOperation) -> bool {
    use RenderOperation::*;
    match operation {
        ClearScreen
        | SetColors(_)
        | JumpToVerticalCenter
        | JumpToRow { .. }
        | JumpToColumn { .. }
        | JumpToBottomRow { .. }
        | InitColumnLayout { .. }
        | EnterColumn { .. }
        | ExitLayout
        | ApplyMargin(_)
        | PopMargin => false,
        RenderText { .. }
        | RenderLineBreak
        | RenderImage(_, _)
        | RenderBlockLine(_)
        | RenderDynamic(_)
        | RenderAsync(_) => true,
    }
}

fn extract_text_lines(operations: &[RenderOperation]) -> Vec<String> {
    let mut output = Vec::new();
    let mut current_line = String::new();
    for operation in operations {
        match operation {
            RenderOperation::RenderText { line, .. } => {
                let text: String = line.iter_texts().map(|text| text.text().content.clone()).collect();
                current_line.push_str(&text);
            }
            RenderOperation::RenderBlockLine(line) => {
                current_line.push_str(&line.prefix.text().content);
                current_line
                    .push_str(&line.text.iter_texts().map(|text| text.text().content.clone()).collect::<String>());
            }
            RenderOperation::RenderLineBreak if !current_line.is_empty() => {
                output.push(mem::take(&mut current_line));
            }
            _ => (),
        };
    }
    if !current_line.is_empty() {
        output.push(current_line);
    }
    output
}

fn extract_slide_text_lines(slide: Slide) -> Vec<String> {
    let operations: Vec<_> = slide.into_operations().into_iter().filter(is_visible).collect();
    extract_text_lines(&operations)
}

#[test]
fn empty_heading_prefix() {
    let input = r#"---
theme:
  override:
    headings:
      h1:
        prefix: ""
---

# hi
"#;
    let mut slides = Test::new(input).build().into_slides();
    let lines = extract_slide_text_lines(slides.remove(0));
    let expected_lines = &["hi"];
    assert_eq!(lines, expected_lines);
}

#[test]
fn prelude_appears_once() {
    let input = r#"---
author: bob
---

# hello

<!-- end_slide -->

# bye
"#;
    let presentation = Test::new(input).build();
    for (index, slide) in presentation.iter_slides().enumerate() {
        let clear_screen_count =
            slide.iter_visible_operations().filter(|op| matches!(op, RenderOperation::ClearScreen)).count();
        let set_colors_count =
            slide.iter_visible_operations().filter(|op| matches!(op, RenderOperation::SetColors(_))).count();
        assert_eq!(clear_screen_count, 1, "{clear_screen_count} clear screens in slide {index}");
        assert_eq!(set_colors_count, 1, "{set_colors_count} clear screens in slide {index}");
    }
}

#[test]
fn slides_start_with_one_newline() {
    let input = r#"---
author: bob
---

# hello

<!-- end_slide -->

# bye
"#;
    let presentation = Test::new(input).build();

    assert_eq!(presentation.iter_slides().count(), 3);

    // Don't process the intro slide as it's special
    let slides = presentation.into_slides().into_iter().skip(1);
    for slide in slides {
        let mut ops = slide.into_operations().into_iter().filter(is_visible);
        // We should start with a newline
        assert!(matches!(ops.next(), Some(RenderOperation::RenderLineBreak)));
        // And the second one should _not_ be a newline
        assert!(!matches!(ops.next(), Some(RenderOperation::RenderLineBreak)));
    }
}

#[test]
fn table() {
    let elements = vec![MarkdownElement::Table(Table {
        header: TableRow(vec![Line::from("key"), Line::from("value"), Line::from("other")]),
        rows: vec![TableRow(vec![Line::from("potato"), Line::from("bar"), Line::from("yes")])],
    })];
    let mut slides = Test::new(elements).build().into_slides();
    let lines = extract_slide_text_lines(slides.remove(0));
    let expected_lines = &["key    │ value │ other", "───────┼───────┼──────", "potato │ bar   │ yes  "];
    assert_eq!(lines, expected_lines);
}

#[test]
fn layout_without_init() {
    let input = "<!-- column: 0 -->";
    Test::new(input).expect_invalid();
}

#[test]
fn already_in_column() {
    let input = r"
<!-- column_layout: [1] -->
<!-- column: 0 -->
<!-- column: 0 -->
";
    Test::new(input).expect_invalid();
}

#[test]
fn column_index_overflow() {
    let input = r"
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
    let input = r"
<!-- column_layout: [1] -->

# hi
";
    Test::new(input).expect_invalid();
}

#[test]
fn end_slide_inside_layout() {
    let input = r"
<!-- column_layout: [1] -->
<!-- end_slide -->
";
    let presentation = Test::new(input).build();
    assert_eq!(presentation.iter_slides().count(), 2);
}

#[test]
fn end_slide_inside_column() {
    let input = r"
<!-- column_layout: [1] -->
<!-- column: 0 -->
<!-- end_slide -->
";
    let presentation = Test::new(input).build();
    assert_eq!(presentation.iter_slides().count(), 2);
}

#[test]
fn pause_inside_layout() {
    let input = r"
<!-- column_layout: [1] -->
<!-- pause -->
<!-- column: 0 -->
";
    let presentation = Test::new(input).build();
    assert_eq!(presentation.iter_slides().count(), 1);
}

#[test]
fn ordered_list_with_pauses() {
    let input = r"
1. one
    1. one_one
    2. one_two

<!-- pause -->

2. two
";
    let mut slides = Test::new(input).build().into_slides();
    let lines = extract_slide_text_lines(slides.remove(0));
    let expected_lines = &["   1. one", "      1. one_one", "      2. one_two", "   2. two"];
    assert_eq!(lines, expected_lines);
}

#[rstest]
#[case::two(2, &["  •  0", "    ◦  00"])]
#[case::three(3, &[" •  0", "    ◦  00"])]
#[case::four(4, &[" •  0", "    ◦  00"])]
fn list_font_size(#[case] font_size: u8, #[case] expected: &[&str]) {
    let input = format!(
        r"
<!-- font_size: {font_size} -->

* 0
    * 00
"
    );
    let options =
        PresentationBuilderOptions { theme_options: ThemeOptions { font_size_supported: true }, ..Default::default() };
    let mut slides = Test::new(input).options(options).build().into_slides();
    let lines = extract_slide_text_lines(slides.remove(0));
    assert_eq!(lines, expected);
}

#[rstest]
#[case::default(Default::default(), 5)]
#[case::no_pause_before(PresentationBuilderOptions{pause_before_incremental_lists: false, ..Default::default()}, 4)]
#[case::no_pause_after(PresentationBuilderOptions{pause_after_incremental_lists: false, ..Default::default()}, 4)]
#[case::no_pauses(
        PresentationBuilderOptions{
            pause_before_incremental_lists: false,
            pause_after_incremental_lists: false,
            ..Default::default()
        },
        3
    )]
fn automatic_pauses(#[case] options: PresentationBuilderOptions, #[case] expected_chunks: usize) {
    let input = r"
<!-- incremental_lists: true -->

* one
    * two
* three

hi
";
    let slides = Test::new(input).options(options).build().into_slides();
    assert_eq!(slides[0].iter_chunks().count(), expected_chunks);
}

#[test]
fn automatic_pauses_no_incremental_lists() {
    let input = "
<!-- incremental_lists: false -->

* one
    * two
* three
        ";
    let options = PresentationBuilderOptions { pause_after_incremental_lists: false, ..Default::default() };
    let slides = Test::new(input).options(options).build().into_slides();
    assert_eq!(slides[0].iter_chunks().count(), 1);
}

#[test]
fn list_item_newlines() {
    let input = "
<!-- list_item_newlines: 3 -->

* one
    * two
";
    let mut slides = Test::new(input).build().into_slides();
    let slide = slides.remove(0);
    let mut ops =
        slide.into_operations().into_iter().skip_while(|op| !matches!(op, RenderOperation::RenderBlockLine { .. }));
    ops.next().expect("no text");
    let newlines =
        ops.position(|op| matches!(op, RenderOperation::RenderBlockLine { .. })).expect("only one text found");
    assert_eq!(newlines, 3);
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
fn incremental_lists_end_of_slide() {
    let input = "
<!-- incremental_lists: true -->

* one
    * two
";
    let slides = Test::new(input).build().into_slides();
    // There shouldn't be an extra one at the end
    assert_eq!(slides[0].iter_chunks().count(), 3);
}

#[test]
fn skip_slide() {
    let input = "
hi

<!-- skip_slide -->
<!-- end_slide -->

bye
";
    let mut slides = Test::new(input).build().into_slides();
    assert_eq!(slides.len(), 1);

    let lines = extract_slide_text_lines(slides.remove(0));
    assert_eq!(lines, &["bye"]);
}

#[test]
fn skip_all_slides() {
    let input = "
hi

<!-- skip_slide -->
";
    let mut slides = Test::new(input).build().into_slides();
    assert_eq!(slides.len(), 1);

    // We should still have one slide but it should be empty
    let lines = extract_slide_text_lines(slides.remove(0));
    assert_eq!(lines, Vec::<String>::new());
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
    let mut slides = Test::new(input).build().into_slides();
    assert_eq!(slides.len(), 1);

    let lines = extract_slide_text_lines(slides.remove(0));
    assert_eq!(lines, &["bye"]);
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
    let mut slides = Test::new(input).options(options).build().into_slides();
    assert_eq!(slides.len(), 1);
    assert_eq!(extract_slide_text_lines(slides.remove(0)), &["bye"]);
}

#[test]
fn pause_after_list() {
    let input = "
1. one

<!-- pause -->

# hi

2. two
";
    let slides = Test::new(input).build().into_slides();
    let first_chunk = &slides[0];
    let operations = first_chunk.iter_visible_operations().collect::<Vec<_>>();
    // This is pretty easy to break, refactor soon
    let last_operation = &operations[operations.len() - 4];
    assert!(matches!(last_operation, RenderOperation::RenderLineBreak), "last operation is {last_operation:?}");
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

    let mut slides = Test::new(input).build().into_slides();
    let operations = slides.remove(0).into_operations();
    let alignments: Vec<_> = operations
        .into_iter()
        .filter_map(|op| match op {
            RenderOperation::RenderText { alignment, .. } => Some(alignment),
            _ => None,
        })
        .collect();
    assert_eq!(
        alignments,
        &[
            Alignment::Left { margin: Default::default() },
            Alignment::Center { minimum_margin: Default::default(), minimum_size: Default::default() },
            Alignment::Right { margin: Default::default() },
        ]
    );
}

#[test]
fn implicit_slide_ends() {
    let input = "
hi
---

hi
---

# hi

<!-- end_slide -->

hi
---
";
    let options = PresentationBuilderOptions { implicit_slide_ends: true, ..Default::default() };
    let slides = Test::new(input).options(options).build().into_slides();
    assert_eq!(slides.len(), 3);
}

#[test]
fn implicit_slide_ends_with_front_matter() {
    let input = "---
theme:
    name: light
---

hi
---
";
    let options = PresentationBuilderOptions { implicit_slide_ends: true, ..Default::default() };
    let slides = Test::new(input).options(options).build().into_slides();
    assert_eq!(slides.len(), 1);
}

#[rstest]
#[case::multiline("hello\nworld")]
#[case::many_open_braces("{{{")]
#[case::many_close_braces("}}}")]
#[case::vim_command("vim: hi")]
#[case::padded_vim_command("vim: hi")]
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
fn extra_fields_in_metadata() {
    let element = MarkdownElement::FrontMatter("nope: 42".into());
    Test::new(vec![element]).expect_invalid();
}

#[test]
fn end_slide_shorthand() {
    let input = "
hola

---

hi
";
    let options = PresentationBuilderOptions { end_slide_shorthand: true, ..Default::default() };
    let presentation = Test::new(input).options(options).build();
    assert_eq!(presentation.iter_slides().count(), 2);

    let second = presentation.iter_slides().nth(1).unwrap();
    let before_text = second.iter_visible_operations().take_while(|e| !matches!(e, RenderOperation::RenderText { .. }));
    let break_count = before_text.filter(|e| matches!(e, RenderOperation::RenderLineBreak)).count();
    assert_eq!(break_count, 1);
}

#[test]
fn parse_front_matter_strict() {
    let options = PresentationBuilderOptions { strict_front_matter_parsing: false, ..Default::default() };
    let elements = vec![MarkdownElement::FrontMatter("potato: yes".into())];
    let result = Test::new(elements).options(options).try_build();
    assert!(result.is_ok());
}

#[rstest]
#[case::enabled(true)]
#[case::disabled(false)]
fn snippet_execution(#[case] enabled: bool) {
    let input = "
```rust +exec
hi
```
";
    let options = PresentationBuilderOptions { enable_snippet_execution: enabled, ..Default::default() };
    let presentation = Test::new(input).options(options).build();
    let slide = presentation.iter_slides().next().unwrap();
    let mut found_render_block = false;
    let mut found_cant_render_block = false;
    for operation in slide.iter_visible_operations() {
        if let RenderOperation::RenderAsync(operation) = operation {
            let operation = format!("{operation:?}");
            if operation.contains("RunSnippetTrigger") {
                assert!(enabled);
                found_render_block = true;
            } else if operation.contains("SnippetExecutionDisabledOperation") {
                assert!(!enabled);
                found_cant_render_block = true;
            }
        }
    }
    if found_render_block {
        assert!(enabled, "snippet execution block found but not enabled");
    } else {
        assert!(!enabled, "snippet execution enabled but not found");
    }
    if found_cant_render_block {
        assert!(!enabled, "can't execute snippet operation found but enabled");
    } else {
        assert!(enabled, "can't execute snippet operation not found");
    }
}

#[test]
fn external_snippet() {
    let temp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = temp.path().file_name().expect("no file name").to_string_lossy();
    let input = format!(
        "
```file +line_numbers +exec
path: {path}
language: rust
```
"
    );
    Test::new(input).build();
}

#[test]
fn footnote() {
    let elements = vec![MarkdownElement::Footnote(Line::from("hi")), MarkdownElement::Footnote(Line::from("bye"))];
    let mut slides = Test::new(elements).build().into_slides();
    let text = extract_slide_text_lines(slides.remove(0));
    assert_eq!(text, &["hi", "bye"]);
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
second
===

![](img.png)
",
    )
    .unwrap();

    fs::write(inner_path.join("foo.txt"), "a").unwrap();

    let input = "
hi

<!-- include: first.md -->
        ";
    let mut slides = Test::new(input).resources_path(path).build().into_slides();
    assert_eq!(slides.len(), 1);

    let text = extract_slide_text_lines(slides.remove(0));
    let expected = &["hi", "first", "second"];
    assert_eq!(text, expected);
}

#[test]
fn self_include() {
    let dir = tempdir().expect("failed to created tempdir");
    let path = dir.path();

    fs::write(path.join("main.md"), "<!-- include: main.md -->").unwrap();
    let input = "<!-- include: main.md -->";

    let err = Test::new(input).resources_path(path).expect_invalid();
    assert!(
        matches!(
            err,
            BuildError::InvalidPresentation {
                error: InvalidPresentation::Import { error: MarkdownSourceError::IncludeCycle(..), .. },
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn include_cycle() {
    let dir = tempdir().expect("failed to created tempdir");
    let path = dir.path();

    fs::write(path.join("main.md"), "<!-- include: inner.md -->").unwrap();
    fs::write(path.join("inner.md"), "<!-- include: main.md -->").unwrap();
    let input = "<!-- include: main.md -->";

    let err = Test::new(input).resources_path(path).expect_invalid();
    assert!(
        matches!(
            err,
            BuildError::InvalidPresentation {
                error: InvalidPresentation::Import { error: MarkdownSourceError::IncludeCycle(..), .. },
                ..
            }
        ),
        "{err:?}"
    );
}
