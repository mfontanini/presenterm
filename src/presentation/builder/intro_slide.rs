use comrak::Arena;

use crate::{
    markdown::{
        elements::{Line, Text},
        parse::MarkdownParser,
        text_style::TextStyle,
    },
    presentation::{
        PresentationMetadata,
        builder::{BuildResult, PresentationBuilder, error::BuildError},
    },
    render::operation::RenderOperation,
    theme::{AuthorPositioning, ElementType},
};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_intro_slide(&mut self, metadata: PresentationMetadata) -> BuildResult {
        let styles = &self.theme.intro_slide;

        let create_text =
            |text: Option<String>, style: TextStyle| -> Option<Text> { text.map(|text| Text::new(text, style)) };
        let title_lines =
            metadata.title.map(|t| self.format_multiline(t, &self.theme.intro_slide.title.style)).transpose()?;

        let sub_title_lines =
            metadata.sub_title.map(|t| self.format_multiline(t, &self.theme.intro_slide.subtitle.style)).transpose()?;
        let event = create_text(metadata.event, styles.event.style);
        let location = create_text(metadata.location, styles.location.style);
        let date = create_text(metadata.date, styles.date.style);
        let authors: Vec<_> = metadata
            .author
            .into_iter()
            .chain(metadata.authors)
            .map(|author| Text::new(author, styles.author.style))
            .collect();
        if !styles.footer {
            self.slide_state.ignore_footer = true;
        }
        self.chunk_operations.push(RenderOperation::JumpToVerticalCenter);
        if let Some(title_lines) = title_lines {
            for line in title_lines {
                self.push_text(line, ElementType::PresentationTitle);
                self.push_line_break();
            }
        }

        if let Some(sub_title_lines) = sub_title_lines {
            for line in sub_title_lines {
                self.push_text(line, ElementType::PresentationTitle);
                self.push_line_break();
            }
        }
        if event.is_some() || location.is_some() || date.is_some() {
            self.push_line_breaks(2);
            if let Some(event) = event {
                self.push_intro_slide_text(event, ElementType::PresentationEvent);
            }
            if let Some(location) = location {
                self.push_intro_slide_text(location, ElementType::PresentationLocation);
            }
            if let Some(date) = date {
                self.push_intro_slide_text(date, ElementType::PresentationDate);
            }
        }
        if !authors.is_empty() {
            match self.theme.intro_slide.author.positioning {
                AuthorPositioning::BelowTitle => {
                    self.push_line_breaks(3);
                }
                AuthorPositioning::PageBottom => {
                    self.chunk_operations.push(RenderOperation::JumpToBottomRow { index: authors.len() as u16 - 1 });
                }
            };
            for author in authors {
                self.push_intro_slide_text(author, ElementType::PresentationAuthor);
            }
        }
        self.slide_state.title = Some(Line::from("[Introduction]"));
        self.terminate_slide();
        Ok(())
    }

    fn push_intro_slide_text(&mut self, text: Text, element_type: ElementType) {
        self.push_text(Line::from(text), element_type);
        self.push_line_break();
    }

    fn format_multiline(&self, text: String, style: &TextStyle) -> Result<Vec<Line>, BuildError> {
        let arena = Arena::default();
        let parser = MarkdownParser::new(&arena);
        let mut lines = Vec::new();
        for line in text.lines() {
            let line = parser.parse_inlines(line).map_err(|e| BuildError::PresentationTitle(e.to_string()))?;
            let mut line = line.resolve(&self.theme.palette)?;
            line.apply_style(style);
            lines.push(line);
        }
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use crate::{presentation::builder::utils::Test, theme::raw};

    #[test]
    fn multiline_centered_title() {
        let input = "---
title: |
    Beep
    Boop boop
---
";
        let theme = raw::PresentationTheme {
            intro_slide: raw::IntroSlideStyle {
                title: raw::IntroSlideTitleStyle {
                    alignment: Some(raw::Alignment::Center { minimum_margin: raw::Margin::Fixed(2), minimum_size: 1 }),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = Test::new(input).theme(theme).render().rows(7).columns(16).advances(0).into_lines();
        let expected = &[
            "                ",
            "                ",
            "      Beep      ",
            "   Boop boop    ",
            "                ",
            "                ",
            "                ",
        ];
        assert_eq!(lines, expected);
    }
}
