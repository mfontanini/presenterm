use crate::{
    ImageRegistry, MarkdownParser, PresentationBuilderOptions, Resources, ThemeOptions, Themes, ThirdPartyRender,
    code::execute::SnippetExecutor,
    commands::{
        keyboard::{CommandKeyBindings, KeyboardListener},
        listener::Command,
    },
    markdown::elements::MarkdownElement,
    presentation::{
        Presentation,
        builder::{BuildError, PresentationBuilder},
    },
    render::TerminalDrawer,
    terminal::emulator::TerminalEmulator,
    theme::raw::PresentationTheme,
};
use std::{io, rc::Rc};

const PRESENTATION: &str = r#"
# Header 1
## Header 2
### Header 3
#### Header 4
##### Header 5
###### Header 6

```rust
fn greet(name: &str) -> String {
    format!("hi {name}")
}
````

* **bold text**
* _italics_
    * `some inline code`
    * ~strikethrough~

> a block quote

<!-- end_slide -->
<!-- end_slide -->
"#;

pub struct ThemesDemo {
    themes: Themes,
    input: KeyboardListener,
    drawer: TerminalDrawer,
}

impl ThemesDemo {
    pub fn new(themes: Themes, bindings: CommandKeyBindings) -> io::Result<Self> {
        let input = KeyboardListener::new(bindings);
        let drawer = TerminalDrawer::new(Default::default(), Default::default())?;
        Ok(Self { themes, input, drawer })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let arena = Default::default();
        let parser = MarkdownParser::new(&arena);
        let elements = parser.parse(PRESENTATION).expect("broken demo presentation");
        let mut presentations = Vec::new();
        for theme_name in self.themes.presentation.theme_names() {
            let theme = self.themes.presentation.load_by_name(&theme_name).expect("theme not found");
            let presentation = self.build(&elements, &theme_name, &theme)?;
            presentations.push(presentation);
        }
        let mut current = 0;
        loop {
            self.drawer.render_operations(presentations[current].current_slide().iter_visible_operations())?;

            let command = self.next_command()?;
            match command {
                DemoCommand::Next => current = (current + 1).min(presentations.len() - 1),
                DemoCommand::Previous => current = current.saturating_sub(1),
                DemoCommand::First => current = 0,
                DemoCommand::Last => current = presentations.len() - 1,
                DemoCommand::Exit => return Ok(()),
            };
        }
    }

    fn next_command(&mut self) -> io::Result<DemoCommand> {
        loop {
            let mut command = self.input.next_command()?;
            while command.is_none() {
                command = self.input.next_command()?;
            }
            match command.unwrap() {
                Command::Next => return Ok(DemoCommand::Next),
                Command::Previous => return Ok(DemoCommand::Previous),
                Command::FirstSlide => return Ok(DemoCommand::First),
                Command::LastSlide => return Ok(DemoCommand::Last),
                Command::Exit => return Ok(DemoCommand::Exit),
                _ => continue,
            }
        }
    }

    fn build(
        &self,
        base_elements: &[MarkdownElement],
        theme_name: &str,
        theme: &PresentationTheme,
    ) -> Result<Presentation, BuildError> {
        let image_registry = ImageRegistry::default();
        let resources = Resources::new("non_existent", "non_existent", image_registry.clone());
        let mut third_party = ThirdPartyRender::default();
        let options = PresentationBuilderOptions {
            theme_options: ThemeOptions { font_size_supported: TerminalEmulator::capabilities().font_size },
            ..Default::default()
        };
        let executer = Rc::new(SnippetExecutor::default());
        let bindings_config = Default::default();
        let builder = PresentationBuilder::new(
            theme,
            resources,
            &mut third_party,
            executer,
            &self.themes,
            image_registry,
            bindings_config,
            options,
        )?;
        let mut elements = vec![MarkdownElement::SetexHeading { text: format!("theme: {theme_name}").into() }];
        elements.extend(base_elements.iter().cloned());
        builder.build(elements)
    }
}

enum DemoCommand {
    Next,
    Previous,
    First,
    Last,
    Exit,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn demo_presentation() {
        let arena = Default::default();
        let parser = MarkdownParser::new(&arena);
        parser.parse(PRESENTATION).expect("broken demo presentation");
    }
}
