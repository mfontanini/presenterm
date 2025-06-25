use crate::{
    config::OptionsConfig,
    presentation::{
        PresentationMetadata, PresentationThemeMetadata,
        builder::{BuildResult, PresentationBuilder, error::BuildError},
    },
    theme::PresentationTheme,
};

impl PresentationBuilder<'_, '_> {
    pub(crate) fn process_front_matter(&mut self, contents: &str) -> BuildResult {
        let metadata = match self.options.strict_front_matter_parsing {
            true => serde_yaml::from_str::<StrictPresentationMetadata>(contents).map(PresentationMetadata::from),
            false => serde_yaml::from_str::<PresentationMetadata>(contents),
        };
        let mut metadata = metadata.map_err(|e| BuildError::InvalidMetadata(e.to_string()))?;
        if metadata.author.is_some() && !metadata.authors.is_empty() {
            return Err(BuildError::InvalidMetadata("cannot have both 'author' and 'authors'".into()));
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
            return Err(BuildError::InvalidMetadata("cannot have both theme path and theme name".into()));
        }
        let mut new_theme = None;
        // Only override the theme if we're not forced to use the default one.
        if !self.options.force_default_theme {
            if let Some(theme_name) = &metadata.name {
                let theme = self
                    .themes
                    .presentation
                    .load_by_name(theme_name)
                    .ok_or_else(|| BuildError::InvalidMetadata(format!("theme '{theme_name}' does not exist")))?;
                new_theme = Some(theme);
            }
            if let Some(theme_path) = &metadata.path {
                let mut theme = self.resources.theme(theme_path)?;
                if let Some(name) = &theme.extends {
                    let base = self
                        .themes
                        .presentation
                        .load_by_name(name)
                        .ok_or_else(|| BuildError::InvalidMetadata(format!("extended theme {name} not found")))?;
                    theme = merge_struct::merge(&theme, &base)
                        .map_err(|e| BuildError::InvalidMetadata(format!("invalid theme: {e}")))?;
                }
                new_theme = Some(theme);
            }
        }
        if let Some(overrides) = &metadata.overrides {
            if overrides.extends.is_some() {
                return Err(BuildError::InvalidMetadata("theme overrides can't use 'extends'".into()));
            }
            let base = new_theme.as_ref().unwrap_or(self.default_raw_theme);
            // This shouldn't fail as the models are already correct.
            let theme = merge_struct::merge(base, overrides)
                .map_err(|e| BuildError::InvalidMetadata(format!("invalid theme: {e}")))?;
            new_theme = Some(theme);
        }
        if let Some(theme) = new_theme {
            self.theme = PresentationTheme::new(&theme, &self.resources, &self.options.theme_options)?;
        }
        Ok(())
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
