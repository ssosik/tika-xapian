//TODO Fix
use crate::tika_document::TikaDocument;
#[allow(unused_imports)]
use std::{fs, io, io::Read, path::Path};
#[allow(unused_imports)]
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
#[allow(unused_imports)]
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

// TODO move terminal stuff into here
//pub(crate) fn NewTerminal() -> Result<Terminal, Report> {
//    // Terminal initialization
//    let stdout = io::stdout().into_raw_mode()?;
//    let stdout = MouseTerminal::from(stdout);
//    let stdout = AlternateScreen::from(stdout);
//    let backend = TermionBackend::new(stdout);
//    let mut terminal = Terminal::new(backend)?;
//    Ok(terminal)
//}

/// TerminalApp holds the state of the application
pub(crate) struct TerminalApp {
    /// Current value of the input box
    pub(crate) input: String,
    /// Query Matches
    pub(crate) matches: Vec<TikaDocument>,
    /// Keep track of which matches are selected
    pub(crate) state: ListState,
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
