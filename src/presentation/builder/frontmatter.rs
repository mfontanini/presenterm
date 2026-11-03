use crate::{
    config::OptionsConfig,
    markdown::{
        elements::{Line, Text},
        parse::MarkdownParser,
        text_style::TextStyle,
    },
    presentation::{
        PresentationMetadata, PresentationThemeMetadata,
        builder::{
            BuildResult, ErrorContextBuilder, PresentationBuilder,
            error::{BuildError, FormatError},
        },
    },
    render::operation::RenderOperation,
    theme::{AuthorPositioning, ElementType, PresentationTheme},
};
use comrak::Arena;

impl PresentationBuilder<'_, '_> {
    pub(crate) fn process_front_matter(&mut self, contents: &str) -> BuildResult {
        let metadata = match self.options.strict_front_matter_parsing {
            true => serde_yaml::from_str::<StrictPresentationMetadata>(contents).map(PresentationMetadata::from),
            false => serde_yaml::from_str::<PresentationMetadata>(contents),
        };
        let mut metadata = metadata.map_err(|e| BuildError::InvalidFrontmatter(e.to_string().format_error()))?;
        if metadata.author.is_some() && !metadata.authors.is_empty() {
            return Err(BuildError::InvalidFrontmatter(
                ErrorContextBuilder::new("authors:", "cannot have both 'author' and 'authors'").build(),
            ));
        }

        if let Some(options) = metadata.options.take() {
            self.options.merge(options);
        }

        {
            let footer_context = &mut self.footer_vars;
            footer_context.title.clone_from(&metadata.title);
            footer_context.sub_title.clone_from(&metadata.sub_title);
            footer_context.location.clone_from(&metadata.location);
            footer_context.event.clone_from(&metadata.event);
            footer_context.date.clone_from(&metadata.date);
            footer_context.author.clone_from(&metadata.author);
        }

        self.set_theme(&metadata.theme)?;
        if metadata.has_frontmatter() {
            self.push_slide_prelude();
            self.push_intro_slide(metadata)?;
        }
        Ok(())
    }

    fn set_theme(&mut self, metadata: &PresentationThemeMetadata) -> BuildResult {
        if metadata.name.is_some() && metadata.path.is_some() {
            return Err(BuildError::InvalidFrontmatter(
                ErrorContextBuilder::new("path:", "cannot set both 'theme.path' and 'theme.name'").build(),
            ));
        }
        let mut new_theme = None;
        // Only override the theme if we're not forced to use the default one.
        if !self.options.force_default_theme {
            if let Some(theme_name) = &metadata.name {
                let theme = self.themes.presentation.load_by_name(theme_name).ok_or_else(|| {
                    BuildError::InvalidFrontmatter(
                        ErrorContextBuilder::new(&format!("name: {theme_name}"), "theme does not exist")
                            .column(7)
                            .build(),
                    )
                })?;
                new_theme = Some(theme);
            }
            if let Some(theme_path) = &metadata.path {
                let mut theme = self.resources.theme(theme_path)?;
                if let Some(name) = &theme.extends {
                    let base = self.themes.presentation.load_by_name(name).ok_or_else(|| {
                        BuildError::InvalidFrontmatter(
                            ErrorContextBuilder::new(&format!("extends: {name}"), "extended theme does not exist")
                                .column(10)
                                .build(),
                        )
                    })?;
                    theme = merge_struct::merge(&theme, &base)
                        .map_err(|e| BuildError::InvalidFrontmatter(format!("malformed theme: {e}")))?;
                }
                new_theme = Some(theme);
            }
        }
        if let Some(overrides) = &metadata.overrides {
            if let Some(extends) = &overrides.extends {
                return Err(BuildError::InvalidFrontmatter(
                    ErrorContextBuilder::new(&format!("extends: {extends}"), "theme overrides can't use 'extends'")
                        .build(),
                ));
            }
            let base = new_theme.as_ref().unwrap_or(self.default_raw_theme);
            // This shouldn't fail as the models are already correct.
            let theme = merge_struct::merge(base, overrides)
                .map_err(|e| BuildError::InvalidFrontmatter(format!("malformed theme: {e}")))?;
            new_theme = Some(theme);
        }
        if let Some(theme) = new_theme {
            self.theme = PresentationTheme::new(&theme, &self.resources, &self.options.theme_options)?;
        }
        Ok(())
    }

    fn push_intro_slide(&mut self, metadata: PresentationMetadata) -> BuildResult {
        let styles = &self.theme.intro_slide;

        let create_text =
            |text: Option<String>, style: TextStyle| -> Option<Text> { text.map(|text| Text::new(text, style)) };
        let title_lines = metadata
            .title
            .map(|t| self.format_multiline(t, &self.theme.intro_slide.title.style, "title"))
            .transpose()?;

        let sub_title_lines = metadata
            .sub_title
            .map(|t| self.format_multiline(t, &self.theme.intro_slide.subtitle.style, "sub_title"))
            .transpose()?;
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
                self.push_text(line, ElementType::PresentationSubTitle);
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

    fn format_multiline(
        &self,
        text: String,
        style: &TextStyle,
        attribute: &'static str,
    ) -> Result<Vec<Line>, BuildError> {
        let arena = Arena::default();
        let parser = MarkdownParser::new(&arena);
        let mut lines = Vec::new();
        for line in text.lines() {
            let line = parser.parse_inlines(line).map_err(|e| {
                BuildError::InvalidFrontmatter(
                    ErrorContextBuilder::new(&format!("{attribute}: ..."), &e.to_string())
                        .column(attribute.len() + 3)
                        .build(),
                )
            })?;

            let mut line = line.resolve(&self.theme.palette)?;
            line.apply_style(style);
            lines.push(line);
        }
        Ok(lines)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictPresentationMetadata {
    #[serde(default)]
    title: Option<String>,

    #[serde(default)]
    sub_title: Option<String>,

    #[serde(default)]
    event: Option<String>,

    #[serde(default)]
    location: Option<String>,

    #[serde(default)]
    date: Option<String>,

    #[serde(default)]
    author: Option<String>,

    #[serde(default)]
    authors: Vec<String>,

    #[serde(default)]
    theme: PresentationThemeMetadata,

    #[serde(default)]
    options: Option<OptionsConfig>,
}

impl From<StrictPresentationMetadata> for PresentationMetadata {
    fn from(strict: StrictPresentationMetadata) -> Self {
        let StrictPresentationMetadata { title, sub_title, event, location, date, author, authors, theme, options } =
            strict;
        Self { title, sub_title, event, location, date, author, authors, theme, options }
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
