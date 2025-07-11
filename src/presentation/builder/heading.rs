use crate::{
    markdown::elements::{Line, Text},
    presentation::builder::{BuildResult, LastElement, PresentationBuilder},
    theme::{ElementType, raw::RawColor},
    ui::separator::RenderSeparator,
};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_slide_title(&mut self, text: Vec<Line<RawColor>>) -> BuildResult {
        if self.options.implicit_slide_ends && !matches!(self.slide_state.last_element, LastElement::None) {
            self.terminate_slide();
        }

        let mut style = self.theme.slide_title.clone();
        self.push_line_breaks(style.padding_top as usize);
        for title_line in text {
            let mut title_line = title_line.resolve(&self.theme.palette)?;
            self.slide_state.title.get_or_insert_with(|| title_line.clone());
            if let Some(font_size) = self.slide_state.font_size {
                style.style = style.style.size(font_size);
            }
            title_line.apply_style(&style.style);
            self.push_text(title_line, ElementType::SlideTitle);
            self.push_line_break();
        }

        for _ in 0..style.padding_bottom {
            self.push_line_break();
        }
        if style.separator {
            self.chunk_operations
                .push(RenderSeparator::new(Line::default(), Default::default(), style.style.size).into());
            self.push_line_break();
        }
        self.push_line_break();
        self.slide_state.ignore_element_line_break = true;
        Ok(())
    }

    pub(crate) fn push_heading(&mut self, level: u8, text: Line<RawColor>) -> BuildResult {
        let mut text = text.resolve(&self.theme.palette)?;
        let (element_type, style) = match level {
            1 => (ElementType::Heading1, &self.theme.headings.h1),
            2 => (ElementType::Heading2, &self.theme.headings.h2),
            3 => (ElementType::Heading3, &self.theme.headings.h3),
            4 => (ElementType::Heading4, &self.theme.headings.h4),
            5 => (ElementType::Heading5, &self.theme.headings.h5),
            6 => (ElementType::Heading6, &self.theme.headings.h6),
            other => panic!("unexpected heading level {other}"),
        };
        if let Some(prefix) = &style.prefix {
            if !prefix.is_empty() {
                let mut prefix = prefix.clone();
                prefix.push(' ');
                text.0.insert(0, Text::from(prefix));
            }
        }
        text.apply_style(&style.style);

        self.push_text(text, element_type);
        self.push_line_breaks(self.slide_font_size() as usize);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        markdown::text_style::Color,
        presentation::builder::{PresentationBuilderOptions, utils::Test},
        theme::raw,
    };

    #[test]
    fn slide_title() {
        let input = "
title
===

hi
";
        let color = Color::new(1, 1, 1);
        let theme = raw::PresentationTheme {
            slide_title: raw::SlideTitleStyle {
                separator: true,
                padding_top: Some(1),
                padding_bottom: Some(1),
                colors: raw::RawColors { foreground: None, background: Some(raw::RawColor::Color(color)) },
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(8).columns(5).into_lines();
        let expected = &["     ", "     ", "title", "     ", "—————", "     ", "hi   ", "     "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn centered_slide_title() {
        let input = "
hi
===

";
        let theme = raw::PresentationTheme {
            slide_title: raw::SlideTitleStyle {
                alignment: Some(raw::Alignment::Center { minimum_margin: raw::Margin::Fixed(1), minimum_size: 0 }),
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(3).columns(6).into_lines();
        let expected = &["      ", "  hi  ", "      "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn implicit_slide_ends() {
        let input = "
hi
===

foo

bye
===

bar

";
        let options = PresentationBuilderOptions { implicit_slide_ends: true, ..Default::default() };
        let lines = Test::new(input).options(options).render().rows(4).columns(6).advances(1).into_lines();
        let expected = &["      ", "bye   ", "      ", "bar   "];
        assert_eq!(lines, expected);
    }

    #[test]
    fn headings() {
        let input = "
# A
## B
### C
#### D
##### E
";
        let theme = raw::PresentationTheme {
            headings: raw::HeadingStyles {
                h1: raw::HeadingStyle { prefix: Some("!".to_string()), ..Default::default() },
                h2: raw::HeadingStyle { prefix: Some("@@".to_string()), ..Default::default() },
                h3: raw::HeadingStyle {
                    alignment: Some(raw::Alignment::Center { minimum_margin: raw::Margin::Fixed(1), minimum_size: 0 }),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(10).columns(6).advances(1).into_lines();
        let expected =
            &["      ", "! A   ", "      ", "@@ B  ", "      ", "  C   ", "      ", "D     ", "      ", "E     "];
        assert_eq!(lines, expected);
    }
}
