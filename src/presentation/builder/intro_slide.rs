use crate::{
    markdown::{
        elements::{Line, Text},
        text_style::TextStyle,
    },
    presentation::{
        PresentationMetadata,
        builder::{BuildResult, PresentationBuilder},
    },
    render::operation::RenderOperation,
    theme::{AuthorPositioning, ElementType},
};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn push_intro_slide(&mut self, metadata: PresentationMetadata) -> BuildResult {
        let styles = &self.theme.intro_slide;

        let create_text =
            |text: Option<String>, style: TextStyle| -> Option<Text> { text.map(|text| Text::new(text, style)) };
        let title = metadata.title.map(|t| self.format_presentation_title(t)).transpose()?;

        let sub_title = create_text(metadata.sub_title, styles.subtitle.style);
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
        if let Some(title) = title {
            self.push_text(title, ElementType::PresentationTitle);
            self.push_line_break();
        }

        if let Some(sub_title) = sub_title {
            self.push_intro_slide_text(sub_title, ElementType::PresentationSubTitle);
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
}
