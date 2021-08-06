use crate::tika_document::TikaDocument;
use crate::util::event::{Event, Events};
use crate::xapian_utils;
use color_eyre::Report;
use std::io::{stdout, Write};
use termion::{event::Key, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use xapian_rusty::{QueryParser, Stem};

// Needed to provide `width()` method on String:
// no method named `width` found for struct `std::string::String` in the current scope
use unicode_width::UnicodeWidthStr;

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

pub fn setup_panic() {
    std::panic::set_hook(Box::new(move |x| {
        stdout()
            .into_raw_mode()
            .unwrap()
            .suspend_raw_mode()
            .unwrap();
        write!(
            stdout().into_raw_mode().unwrap(),
            "{}",
            termion::screen::ToMainScreen
        )
        .unwrap();
        write!(stdout(), "{:?}", x).unwrap();
    }));
}

/// Interactive query interface
pub fn interactive_query() -> Result<Vec<String>, Report> {
    //let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;
    let mut qp = QueryParser::new()?;
    let mut stem = Stem::new("en")?;
    qp.set_stemmer(&mut stem)?;

    //let flags = FlagBoolean as i16
    //    | FlagPhrase as i16
    //    | FlagLovehate as i16
    //    | FlagBooleanAnyCase as i16
    //    | FlagWildcard as i16
    //    | FlagPureNot as i16
    //    | FlagPartial as i16
    //    | FlagSpellingCorrection as i16;

    let mut tui = tui::Terminal::new(TermionBackend::new(AlternateScreen::from(
        stdout().into_raw_mode().unwrap(),
    )))
    .unwrap();

    // Setup event handlers
    let events = Events::new();

    // Create default app state
    let mut app = TerminalApp::default();

    loop {
        // Draw UI
        tui.draw(|f| {
            let panes = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                .split(f.size());
            let selected_style = Style::default().add_modifier(Modifier::REVERSED);

            // Output area where match titles are displayed
            let matches: Vec<ListItem> = app
                .matches
                .iter()
                .map(|m| {
                    let content = vec![Spans::from(Span::raw(format!("{}", m.title)))];
                    ListItem::new(content)
                })
                .collect();
            let matches = List::new(matches)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(selected_style);
            //.highlight_symbol("> ");
            f.render_stateful_widget(matches, panes[0], &mut app.state);

            // Input area where queries are entered
            let input = Paragraph::new(app.input.as_ref())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(input, panes[1]);
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                panes[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                panes[1].y + 1,
            )
        })?;

        // Handle input
        if let Event::Input(input) = events.next()? {
            match input {
                Key::Char('\n') => {
                    // Select choice
                    break;
                }
                Key::Ctrl('c') => {
                    break;
                }
                Key::Char(c) => {
                    app.input.push(c);
                }
                Key::Backspace => {
                    app.input.pop();
                }
                Key::Down => {
                    app.next();
                }
                Key::Up => {
                    app.previous();
                }
                _ => {}
            }

            let mut owned_string: String = app.input.to_owned();
            let borrowed_string: &str = "\n";
            owned_string.push_str(borrowed_string);

            let query = xapian_utils::parse_user_query(&owned_string)?;
            //app.matches = xapian_utils::query_db(db, query)?;
            app.matches = xapian_utils::query_db(query)?;
        }
    }

    tui.clear().unwrap();

    Ok(app.get_selected())
}
