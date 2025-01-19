use super::listener::{Command, CommandDiscriminants};
use crate::custom::KeyBindingsConfig;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};
use schemars::JsonSchema;
use serde_with::DeserializeFromStr;
use std::{fmt, io, iter, mem, str::FromStr, time::Duration};

/// A keyboard command listener.
pub struct KeyboardListener {
    bindings: CommandKeyBindings,
    events: Vec<KeyEvent>,
}

impl KeyboardListener {
    pub fn new(bindings: CommandKeyBindings) -> Self {
        Self { bindings, events: Vec::new() }
    }

    /// Polls for the next input command coming from the keyboard.
    pub(crate) fn poll_next_command(&mut self, timeout: Duration) -> io::Result<Option<Command>> {
        if poll(timeout)? { self.next_command() } else { Ok(None) }
    }

    /// Blocks waiting for the next command.
    pub(crate) fn next_command(&mut self) -> io::Result<Option<Command>> {
        let mut events = mem::take(&mut self.events);
        let (command, events) = match read()? {
            // Ignore release events
            Event::Key(event) if event.kind == KeyEventKind::Release => (None, events),
            Event::Key(event) => {
                events.push(event);
                self.match_events(events)
            }
            Event::Resize(..) => (Some(Command::Redraw), events),
            _ => (None, vec![]),
        };
        self.events = events;
        Ok(command)
    }

    fn match_events(&self, events: Vec<KeyEvent>) -> (Option<Command>, Vec<KeyEvent>) {
        match self.bindings.apply(&events) {
            InputAction::Emit(command) => (Some(command), Vec::new()),
            InputAction::Buffer => (None, events),
            InputAction::Reset => (None, Vec::new()),
        }
    }
}

enum InputAction {
    Buffer,
    Reset,
    Emit(Command),
}

pub struct CommandKeyBindings {
    bindings: Vec<(KeyBinding, CommandDiscriminants)>,
}

impl CommandKeyBindings {
    fn apply(&self, events: &[KeyEvent]) -> InputAction {
        let mut any_partials = false;
        for (binding, identifier) in &self.bindings {
            match binding.match_events(events) {
                BindingMatch::Full(context) => return Self::instantiate(identifier, context),
                BindingMatch::Partial => any_partials = true,
                BindingMatch::None => (),
            }
        }
        if any_partials { InputAction::Buffer } else { InputAction::Reset }
    }

    fn instantiate(discriminant: &CommandDiscriminants, context: MatchContext) -> InputAction {
        use CommandDiscriminants::*;
        let command = match discriminant {
            Redraw => Command::Redraw,
            Next => Command::Next,
            NextFast => Command::NextFast,
            Previous => Command::Previous,
            PreviousFast => Command::PreviousFast,
            FirstSlide => Command::FirstSlide,
            LastSlide => Command::LastSlide,
            GoToSlide => {
                match context {
                    // this means the command is malformed and this should have been caught earlier
                    // on.
                    MatchContext::None => return InputAction::Reset,
                    MatchContext::Number(number) => Command::GoToSlide(number),
                }
            }
            RenderAsyncOperations => Command::RenderAsyncOperations,
            Exit => Command::Exit,
            Suspend => Command::Suspend,
            Reload => Command::Reload,
            HardReload => Command::HardReload,
            ToggleSlideIndex => Command::ToggleSlideIndex,
            ToggleKeyBindingsConfig => Command::ToggleKeyBindingsConfig,
            CloseModal => Command::CloseModal,
        };
        InputAction::Emit(command)
    }

    fn validate_conflicts<'a>(
        bindings: impl Iterator<Item = &'a KeyBinding>,
    ) -> Result<(), KeyBindingsValidationError> {
        let mut bindings: Vec<_> = bindings.map(|binding| &binding.0).collect();
        bindings.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for window in bindings.windows(2) {
            if window[0].iter().eq(window[1].iter().take(window[0].len())) {
                return Err(KeyBindingsValidationError::Conflict(
                    KeyBinding(window[0].clone()),
                    KeyBinding(window[1].clone()),
                ));
            }
        }
        Ok(())
    }
}

impl TryFrom<KeyBindingsConfig> for CommandKeyBindings {
    type Error = KeyBindingsValidationError;

    fn try_from(config: KeyBindingsConfig) -> Result<Self, Self::Error> {
        let zip = |discriminant, bindings: Vec<KeyBinding>| bindings.into_iter().zip(iter::repeat(discriminant));
        if !config.go_to_slide.iter().all(|k| k.expects_number()) {
            return Err(KeyBindingsValidationError::Invalid("go_to_slide", "<number> matcher required"));
        }
        let bindings: Vec<_> = iter::empty()
            .chain(zip(CommandDiscriminants::Next, config.next))
            .chain(zip(CommandDiscriminants::NextFast, config.next_fast))
            .chain(zip(CommandDiscriminants::Previous, config.previous))
            .chain(zip(CommandDiscriminants::PreviousFast, config.previous_fast))
            .chain(zip(CommandDiscriminants::FirstSlide, config.first_slide))
            .chain(zip(CommandDiscriminants::LastSlide, config.last_slide))
            .chain(zip(CommandDiscriminants::GoToSlide, config.go_to_slide))
            .chain(zip(CommandDiscriminants::Exit, config.exit))
            .chain(zip(CommandDiscriminants::Suspend, config.suspend))
            .chain(zip(CommandDiscriminants::HardReload, config.reload))
            .chain(zip(CommandDiscriminants::ToggleSlideIndex, config.toggle_slide_index))
            .chain(zip(CommandDiscriminants::ToggleKeyBindingsConfig, config.toggle_bindings))
            .chain(zip(CommandDiscriminants::RenderAsyncOperations, config.execute_code))
            .chain(zip(CommandDiscriminants::CloseModal, config.close_modal))
            .collect();
        Self::validate_conflicts(bindings.iter().map(|binding| &binding.0))?;
        Ok(Self { bindings })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeyBindingsValidationError {
    #[error("invalid binding for {0}: {1}")]
    Invalid(&'static str, &'static str),

    #[error("conflicting keybindings: {0} and {1}")]
    Conflict(KeyBinding, KeyBinding),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BindingMatch {
    Full(MatchContext),
    Partial,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, DeserializeFromStr, JsonSchema)]
pub struct KeyBinding(#[schemars(with = "String")] Vec<KeyMatcher>);

impl KeyBinding {
    fn match_events(&self, mut events: &[KeyEvent]) -> BindingMatch {
        let mut output_context = MatchContext::None;
        for (index, matcher) in self.0.iter().enumerate() {
            let Some((context, rest)) = matcher.try_match_events(events) else {
                return BindingMatch::None;
            };
            if !matches!(context, MatchContext::None) {
                output_context = context;
            }
            events = rest;

            // We ran all matchers but we have no events left; this is a partial match.
            if index != self.0.len() - 1 && events.is_empty() {
                return BindingMatch::Partial;
            }
        }
        // If there's more events than we need, this is an issue on the caller side.
        BindingMatch::Full(output_context)
    }

    fn expects_number(&self) -> bool {
        self.0.iter().any(|m| matches!(m, KeyMatcher::Number))
    }
}

impl FromStr for KeyBinding {
    type Err = KeyBindingParseError;

    fn from_str(mut input: &str) -> Result<Self, Self::Err> {
        let mut matchers = Vec::new();
        let mut has_numbers = false;
        while !input.is_empty() {
            let (matcher, rest) = KeyMatcher::parse(input)?;
            let is_number = matches!(matcher, KeyMatcher::Number);
            // We don't want more than one <number> matcher
            if has_numbers && is_number {
                return Err(KeyBindingParseError::TooManyNumbers);
            }
            has_numbers = has_numbers || is_number;
            matchers.push(matcher);
            input = rest;
        }
        Ok(Self(matchers))
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for matcher in &self.0 {
            write!(f, "{matcher}")?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeyBindingParseError {
    #[error("no input")]
    NoInput,

    #[error("not a valid key: {0}")]
    InvalidKey(char),

    #[error("too many number placeholders")]
    TooManyNumbers,

    #[error("invalid control sequence")]
    InvalidControlSequence,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
enum KeyMatcher {
    Key(KeyCombination),
    Number,
}

impl KeyMatcher {
    fn try_match_events<'a>(&self, events: &'a [KeyEvent]) -> Option<(MatchContext, &'a [KeyEvent])> {
        match self {
            Self::Key(combo) => Self::try_match_key(combo, events),
            Self::Number => Self::try_match_number(events),
        }
    }

    fn try_match_key<'a>(combo: &KeyCombination, events: &'a [KeyEvent]) -> Option<(MatchContext, &'a [KeyEvent])> {
        let event = events.first()?;
        let is_control = event.modifiers == KeyModifiers::CONTROL;
        if combo.key == event.code && combo.control == is_control {
            let rest = &events[1..];
            Some((MatchContext::None, rest))
        } else {
            None
        }
    }

    fn try_match_number(mut events: &[KeyEvent]) -> Option<(MatchContext, &[KeyEvent])> {
        let mut number = None;
        while let Some((head, rest)) = events.split_first() {
            let digit = match head.code {
                KeyCode::Char(c) if c.is_ascii_digit() => c.to_digit(10).expect("not a digit"),
                _ => break,
            };

            let next = number.unwrap_or(0u32).checked_mul(10).and_then(|number| number.checked_add(digit));
            match next {
                Some(n) => {
                    number = Some(n);
                    events = rest;
                }
                // if we overflow we're done
                None => return None,
            }
        }
        number.map(|number| (MatchContext::Number(number), events))
    }

    fn parse(input: &str) -> Result<(Self, &str), KeyBindingParseError> {
        if let Some(input) = input.strip_prefix("<number>") {
            Ok((Self::Number, input))
        } else if let Some(input) = Self::try_match_input(input, &["<c-", "<C-"]) {
            let (key, input) = Self::parse_key_code(input)?;
            let Some(input) = input.strip_prefix('>') else {
                return Err(KeyBindingParseError::InvalidControlSequence);
            };
            let matcher = Self::Key(KeyCombination { key, control: true });
            Ok((matcher, input))
        } else {
            let (key, input) = Self::parse_key_code(input)?;
            let matcher = Self::Key(KeyCombination { key, control: false });
            Ok((matcher, input))
        }
    }

    fn parse_key_code(input: &str) -> Result<(KeyCode, &str), KeyBindingParseError> {
        if let Some(input) = Self::try_match_input(input, &["<PageUp>", "<page_up>"]) {
            Ok((KeyCode::PageUp, input))
        } else if let Some(input) = Self::try_match_input(input, &["<PageDown>", "<page_down>"]) {
            Ok((KeyCode::PageDown, input))
        } else if let Some(input) = Self::try_match_input(input, &["<cr>", "<CR>", "<Enter>", "<enter>"]) {
            Ok((KeyCode::Enter, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Home>", "<home>"]) {
            Ok((KeyCode::Home, input))
        } else if let Some(input) = Self::try_match_input(input, &["<End>", "<end>"]) {
            Ok((KeyCode::End, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Left>", "<left>"]) {
            Ok((KeyCode::Left, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Right>", "<right>"]) {
            Ok((KeyCode::Right, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Up>", "<up>"]) {
            Ok((KeyCode::Up, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Down>", "<down>"]) {
            Ok((KeyCode::Down, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Esc>", "<esc>"]) {
            Ok((KeyCode::Esc, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Tab>", "<tab>"]) {
            Ok((KeyCode::Tab, input))
        } else if let Some(input) = Self::try_match_input(input, &["<Backspace>", "<backspace>"]) {
            Ok((KeyCode::Backspace, input))
        } else if let Some(input) = Self::try_match_input(input, &["<F", "<f"]) {
            let (number, rest) = input.split_once('>').ok_or(KeyBindingParseError::InvalidControlSequence)?;
            let number: u8 = number.parse().map_err(|_| KeyBindingParseError::InvalidControlSequence)?;
            if number > 12 { Err(KeyBindingParseError::InvalidControlSequence) } else { Ok((KeyCode::F(number), rest)) }
        } else {
            let next = input.chars().next().ok_or(KeyBindingParseError::NoInput)?;
            // don't allow these as they create ambiguity
            if next == '<' || next == '>' {
                Err(KeyBindingParseError::InvalidKey(next))
            } else if next.is_alphanumeric() || next.is_ascii_punctuation() || next == ' ' {
                let key = KeyCode::Char(next);
                Ok((key, &input[next.len_utf8()..]))
            } else {
                Err(KeyBindingParseError::InvalidKey(next))
            }
        }
    }

    fn try_match_input<'a>(input: &'a str, aliases: &[&str]) -> Option<&'a str> {
        for alias in aliases {
            if let Some(input) = input.strip_prefix(alias) {
                return Some(input);
            }
        }
        None
    }
}

impl fmt::Display for KeyMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number => write!(f, "<number>"),
            Self::Key(combo) => {
                if combo.control {
                    write!(f, "<c-")?;
                }
                match combo.key {
                    KeyCode::Char(' ') => write!(f, "' '")?,
                    KeyCode::Char(c) => write!(f, "{}", c)?,
                    other => write!(f, "<{other:?}>")?,
                };
                if combo.control {
                    write!(f, ">")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MatchContext {
    Number(u32),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
struct KeyCombination {
    key: KeyCode,
    control: bool,
}

impl KeyCombination {
    #[cfg(test)]
    fn char(c: char) -> Self {
        Self { key: KeyCode::Char(c), control: false }
    }

    #[cfg(test)]
    fn control_char(c: char) -> Self {
        Self { key: KeyCode::Char(c), control: true }
    }
}

impl From<KeyCode> for KeyCombination {
    fn from(key: KeyCode) -> Self {
        Self { key, control: false }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crossterm::event::KeyEventState;
    use rstest::rstest;

    trait KeyEventSource {
        fn into_event(self) -> KeyEvent;
    }

    impl KeyEventSource for KeyCode {
        fn into_event(self) -> KeyEvent {
            KeyEvent {
                code: self,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }
        }
    }

    impl KeyEventSource for char {
        fn into_event(self) -> KeyEvent {
            KeyCode::Char(self).into_event()
        }
    }

    trait KeyEventExt {
        fn with_control(self) -> Self;
    }

    impl KeyEventExt for KeyEvent {
        fn with_control(mut self) -> Self {
            self.modifiers = KeyModifiers::CONTROL;
            self
        }
    }

    #[rstest]
    #[case::number("<number>", vec![KeyMatcher::Number])]
    #[case::char("w", vec![KeyMatcher::Key(KeyCombination::char('w'))])]
    #[case::ctrl_char1("<c-w>", vec![KeyMatcher::Key(KeyCombination::control_char('w'))])]
    #[case::ctrl_char2("<C-w>", vec![KeyMatcher::Key(KeyCombination::control_char('w'))])]
    #[case::dot(".", vec![KeyMatcher::Key(KeyCombination::char('.'))])]
    #[case::dot(" ", vec![KeyMatcher::Key(KeyCombination::char(' '))])]
    #[case::multi("hi", vec![KeyMatcher::Key(KeyCombination::char('h')), KeyMatcher::Key(KeyCombination::char('i'))])]
    #[case::page_up1("<page_up>", vec![KeyMatcher::Key(KeyCode::PageUp.into())])]
    #[case::page_up2("<PageUp>", vec![KeyMatcher::Key(KeyCode::PageUp.into())])]
    #[case::page_down1("<page_down>", vec![KeyMatcher::Key(KeyCode::PageDown.into())])]
    #[case::page_down2("<PageDown>", vec![KeyMatcher::Key(KeyCode::PageDown.into())])]
    #[case::enter1("<CR>", vec![KeyMatcher::Key(KeyCode::Enter.into())])]
    #[case::enter2("<cr>", vec![KeyMatcher::Key(KeyCode::Enter.into())])]
    #[case::enter3("<enter>", vec![KeyMatcher::Key(KeyCode::Enter.into())])]
    #[case::home1("<home>", vec![KeyMatcher::Key(KeyCode::Home.into())])]
    #[case::home2("<Home>", vec![KeyMatcher::Key(KeyCode::Home.into())])]
    #[case::end1("<End>", vec![KeyMatcher::Key(KeyCode::End.into())])]
    #[case::end2("<end>", vec![KeyMatcher::Key(KeyCode::End.into())])]
    #[case::left1("<Left>", vec![KeyMatcher::Key(KeyCode::Left.into())])]
    #[case::left2("<left>", vec![KeyMatcher::Key(KeyCode::Left.into())])]
    #[case::right1("<Right>", vec![KeyMatcher::Key(KeyCode::Right.into())])]
    #[case::right2("<right>", vec![KeyMatcher::Key(KeyCode::Right.into())])]
    #[case::up1("<Up>", vec![KeyMatcher::Key(KeyCode::Up.into())])]
    #[case::up2("<up>", vec![KeyMatcher::Key(KeyCode::Up.into())])]
    #[case::down1("<Down>", vec![KeyMatcher::Key(KeyCode::Down.into())])]
    #[case::down2("<down>", vec![KeyMatcher::Key(KeyCode::Down.into())])]
    #[case::esc1("<Esc>", vec![KeyMatcher::Key(KeyCode::Esc.into())])]
    #[case::esc2("<esc>", vec![KeyMatcher::Key(KeyCode::Esc.into())])]
    #[case::f1("<f1>", vec![KeyMatcher::Key(KeyCode::F(1).into())])]
    #[case::f12("<f12>", vec![KeyMatcher::Key(KeyCode::F(12).into())])]
    #[case::backspace1("<Backspace>", vec![KeyMatcher::Key(KeyCode::Backspace.into())])]
    #[case::backspace2("<backspace>", vec![KeyMatcher::Key(KeyCode::Backspace.into())])]
    #[case::tab1("<Tab>", vec![KeyMatcher::Key(KeyCode::Tab.into())])]
    #[case::tab2("<tab>", vec![KeyMatcher::Key(KeyCode::Tab.into())])]
    fn parse_key_binding(#[case] pattern: &str, #[case] matchers: Vec<KeyMatcher>) {
        let binding = KeyBinding::from_str(pattern).expect("failed to parse");
        let expected = KeyBinding(matchers);
        assert_eq!(binding, expected);
    }

    #[rstest]
    #[case::invalid_tag("<hi>")]
    #[case::invalid_char("🚀")]
    #[case::too_many_numbers("<number><number>")]
    #[case::control_sequence("<C-w")]
    #[case::f10("<f13>")]
    #[case::unfinished_f("<f1")]
    fn invalid_key_bindings(#[case] input: &str) {
        let result = KeyBinding::from_str(input);
        assert!(result.is_err(), "not an error");
    }

    #[rstest]
    #[case::single("g", &['g'.into_event()])]
    #[case::single_uppercase("G", &['G'.into_event()])]
    #[case::multi("gg", &['g'.into_event(), 'g'.into_event()])]
    #[case::multi_space(" g", &[' '.into_event(), 'g'.into_event()])]
    #[case::control("<c-w>", &['w'.into_event().with_control()])]
    #[case::page_up("<PageUp>", &[KeyCode::PageUp.into_event()])]
    #[case::page_down("<PageDown>", &[KeyCode::PageDown.into_event()])]
    #[case::enter("<Enter>", &[KeyCode::Enter.into_event()])]
    #[case::home("<Home>", &[KeyCode::Home.into_event()])]
    #[case::end("<End>", &[KeyCode::End.into_event()])]
    fn matching(#[case] pattern: &str, #[case] events: &[KeyEvent]) {
        let binding = KeyBinding::from_str(pattern).expect("failed to parse");
        let result = binding.match_events(events);
        assert!(matches!(result, BindingMatch::Full(_)), "not full match: {result:?}");
    }

    #[rstest]
    #[case::fewer("gg", &['g'.into_event()])]
    #[case::number_something1("<number>G", &['4'.into_event()])]
    #[case::number_something2("<number>G", &['4'.into_event(), '2'.into_event()])]
    #[case::number_something3(":<number><CR>", &[':'.into_event(), '4'.into_event()])]
    fn partial_matching(#[case] pattern: &str, #[case] events: &[KeyEvent]) {
        let binding = KeyBinding::from_str(pattern).expect("failed to parse");
        let result = binding.match_events(events);
        assert!(matches!(result, BindingMatch::Partial), "not partial match: {result:?}");
    }

    #[rstest]
    #[case::number_something("<number>G", &['4'.into_event(), 'K'.into_event()])]
    fn no_matching(#[case] pattern: &str, #[case] events: &[KeyEvent]) {
        let binding = KeyBinding::from_str(pattern).expect("failed to parse");
        let result = binding.match_events(events);
        assert!(matches!(result, BindingMatch::None), "some match: {result:?}");
    }

    #[rstest]
    #[case::number_something("<number>G", &['4'.into_event(), '2'.into_event(), 'G'.into_event()])]
    #[case::number_something(
        ":<number><cr>",
        &[':'.into_event(), '4'.into_event(), '2'.into_event(), KeyCode::Enter.into_event()]
    )]
    fn match_number(#[case] pattern: &str, #[case] events: &[KeyEvent]) {
        let binding = KeyBinding::from_str(pattern).expect("failed to parse");
        let result = binding.match_events(events);
        let BindingMatch::Full(MatchContext::Number(number)) = result else {
            panic!("unexpected match: {result:?}");
        };
        assert_eq!(number, 42);
    }

    #[rstest]
    #[case(&["<number>G", "other", "<number>Go"])]
    #[case(&["<PageUp><PageDown>", "something", "<PageUp>"])]
    #[case(&["<cr><cr>", "<cr><cr>"])]
    #[case(&["<c-w>", "<c-w>a"])]
    #[case(&["<c-w>", "<c-w>"])]
    #[case(&["<number>", "<number>"])]
    fn conflicts(#[case] patterns: &[&str]) {
        let bindings: Vec<_> = patterns.iter().map(|p| KeyBinding::from_str(p).unwrap()).collect();
        let result = CommandKeyBindings::validate_conflicts(bindings.iter());
        assert!(result.is_err(), "not an error: {result:?}");
    }

    #[rstest]
    #[case(&["<number>Ga", "<number>Go"])]
    #[case(&["<c-a><number>", "<c-a>hi"])]
    fn no_conflicts(#[case] patterns: &[&str]) {
        let bindings: Vec<_> = patterns.iter().map(|p| KeyBinding::from_str(p).unwrap()).collect();
        let result = CommandKeyBindings::validate_conflicts(bindings.iter());
        assert!(result.is_ok(), "got error: {result:?}");
    }

    #[rstest]
    #[case("<number>G")]
    #[case("<PageUp>potato")]
    #[case("<Esc><number><PageUp>")]
    fn display(#[case] pattern: &str) {
        let binding = KeyBinding::from_str(pattern).expect("invalid pattern");
        let rendered = binding.to_string();
        assert_eq!(rendered, pattern);
    }
}
