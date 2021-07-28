use crate::tika_document::TikaDocument;
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

/// TerminalApp holds the state of the application
pub(crate) struct TerminalApp {
    /// Current value of the input box
    input: String,
    /// Query Matches
    matches: Vec<TikaDocument>,
    /// Keep track of which matches are selected
    state: ListState,
}

impl TerminalApp {
    pub fn get_selected(&mut self) -> Vec<String> {
        let mut ret: Vec<String> = Vec::new();
        if let Some(i) = self.state.selected() {
            if let Some(s) = self.matches[i].full_path.to_str() {
                ret.push(s.into());
            }
        };
        ret
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.matches.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.matches.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Default for TerminalApp {
    fn default() -> TerminalApp {
        TerminalApp {
            input: String::new(),
            matches: Vec::new(),
            state: ListState::default(),
        }
    }
}
