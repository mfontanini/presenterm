use super::*;
use crate::presentation::builder::utils::Test;

#[test]
fn prelude_appears_once() {
    let input = "---
author: bob
---

# hello

<!-- end_slide -->

# bye
";
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
    // land in first slide after into
    let lines = Test::new(input).render().rows(2).columns(5).advances(1).into_lines();
    assert_eq!(lines, &["     ", "hello"]);

    // land in second one
    let lines = Test::new(input).render().rows(2).columns(5).advances(2).into_lines();
    assert_eq!(lines, &["     ", "bye  "]);
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
    // first slide
    let options = PresentationBuilderOptions { end_slide_shorthand: true, ..Default::default() };
    let lines = Test::new(input).options(options.clone()).render().rows(2).columns(5).into_lines();
    assert_eq!(lines, &["     ", "hola "]);

    // second slide
    let lines = Test::new(input).options(options).render().rows(2).columns(5).advances(1).into_lines();
    assert_eq!(lines, &["     ", "hi   "]);
}

#[test]
fn parse_front_matter_strict() {
    let options = PresentationBuilderOptions { strict_front_matter_parsing: false, ..Default::default() };
    let elements = vec![MarkdownElement::FrontMatter("potato: yes".into())];
    let result = Test::new(elements).options(options).try_build();
    assert!(result.is_ok());
}

#[test]
fn footnote() {
    let elements = vec![MarkdownElement::Footnote(Line::from("hi")), MarkdownElement::Footnote(Line::from("bye"))];
    let lines = Test::new(elements).render().rows(3).columns(5).into_lines();
    let expected = &["     ", "hi   ", "bye  "];
    assert_eq!(lines, expected);
}
