mod tika_document;
mod tui_app;
mod util;

use crate::tika_document::{parse_file, TikaDocument};
use crate::util::event::{Event, Events};
use crate::util::glob_files;
use clap::{App, Arg, ArgMatches, SubCommand};
use color_eyre::Report;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagLovehate, FlagPartial, FlagPhrase, FlagPureNot,
    FlagSpellingCorrection, FlagWildcard,
};
use xapian_rusty::{
    Database, Document, Query, QueryParser, Stem, TermGenerator, WritableDatabase, XapianOp, BRASS,
    DB_CREATE_OR_OPEN, DB_CREATE_OR_OVERWRITE,
};

// Needed to provide `width()` method on String:
// no method named `width` found for struct `std::string::String` in the current scope
use unicode_width::UnicodeWidthStr;

fn setup<'a>(default_config_file: &str) -> Result<ArgMatches, Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    let cli = App::new("tika")
        .version("1.0")
        .author("Steve <steve@little-fluffy.cloud>")
        .about("Things I Know About: Zettlekasten-like Markdown+FrontMatter Indexer and query tool")
        .arg(
            Arg::with_name("config")
                .short("c")
                .value_name("FILE")
                .help(
                    format!(
                        "Point to a config TOML file, defaults to `{}`",
                        default_config_file
                    )
                    .as_str(),
                )
                .default_value(&default_config_file)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("index")
                .short("i")
                .help("Index data rather than querying the DB"),
        )
        .arg(
            Arg::with_name("source")
                .short("s")
                .value_name("DIRECTORY")
                .help("Glob path to markdown files to load")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("query")
                .about("Query the index")
                .arg(Arg::with_name("query").required(true).help("Query string")),
        )
        .get_matches();

    Ok(cli)
}

fn main() -> Result<(), Report> {
    let default_config_file = shellexpand::tilde("~/.config/tika/tika.toml");
    let cli = setup(&default_config_file)?;

    // If requested, index the data
    if cli.occurrences_of("index") > 0 {
        let mut db = WritableDatabase::new("mydb", BRASS, DB_CREATE_OR_OPEN)?;
        let mut tg = TermGenerator::new()?;
        let mut stemmer = Stem::new("en")?;
        tg.set_stemmer(&mut stemmer)?;

        ////let (matches, errors): (Vec<_>, Vec<_>) = glob_files(
        //glob_files(
        //    &cli.value_of("config").unwrap(),
        //    cli.value_of("source"),
        //    cli.occurrences_of("v") as i8,
        //)
        //    .into_iter()
        //.map(|entry| match entry {
        //    Ok(path) => {
        //            if let Ok(tikadoc) = parse_file(&path) {
        //                perform_index(&mut db, &mut tg, &tikadoc)?;
        //                if cli.occurrences_of("v") > 0 {
        //                    if let Ok(p) = tikadoc.full_path.into_string() {
        //                        println!("✅ {}", p);
        //                    }
        //                }
        //            } else {
        //                eprintln!("❌ Failed to load file {}", path.display());
        //            }
        //    }
        //    Err(e) => println!("{:?}", e),
        //})
        //.partition(Result::is_ok);

        for entry in glob_files(
            &cli.value_of("config").unwrap(),
            cli.value_of("source"),
            cli.occurrences_of("v") as i8,
        )
        .expect("Failed to read glob pattern")
        {
            match entry {
                // TODO convert this to iterator style using map/filter
                Ok(path) => {
                    if let Ok(tikadoc) = parse_file(&path) {
                        perform_index(&mut db, &mut tg, &tikadoc)?;
                        if cli.occurrences_of("v") > 0 {
                            //if let Ok(p) = tikadoc.full_path.into_string() {
                            //    println!("✅ {}", p);
                            //}
                            println!("✅ {}", tikadoc.filename);
                        }
                    } else {
                        eprintln!("❌ Failed to load file {}", path.display());
                    }
                }

                Err(e) => eprintln!("❌ {:?}", e),
            }
        }

        db.commit()?;
    }

    //query()?;
    interactive_query()?;

    Ok(())
}

fn perform_index(
    db: &mut WritableDatabase,
    tg: &mut TermGenerator,
    tikadoc: &TikaDocument,
) -> Result<(), Report> {
    // Create a new Xapian Document to store attributes on the passed-in TikaDocument
    let mut doc = Document::new()?;
    tg.set_document(&mut doc)?;

    tg.index_text_with_prefix(&tikadoc.author, "A")?;
    tg.index_text_with_prefix(&tikadoc.date_str()?, "D")?;
    tg.index_text_with_prefix(&tikadoc.filename, "F")?;
    tg.index_text_with_prefix(&tikadoc.full_path.clone().into_string().unwrap(), "F")?;
    tg.index_text_with_prefix(&tikadoc.title, "S")?;
    tg.index_text_with_prefix(&tikadoc.subtitle, "XS")?;
    for tag in &tikadoc.tags {
        tg.index_text_with_prefix(&tag, "K")?;
    }

    tg.index_text(&tikadoc.body)?;

    // Convert the TikaDocument into JSON and set it in the DB for retrieval later
    doc.set_data(&serde_json::to_string(&tikadoc).unwrap())?;

    let id = "Q".to_owned() + &tikadoc.filename;
    doc.add_boolean_term(&id)?;
    db.replace_document(&id, &mut doc)?;

    Ok(())
}

fn query() -> Result<(), Report> {
    let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;
    let mut qp = QueryParser::new()?;
    let mut stem = Stem::new("en")?;
    qp.set_stemmer(&mut stem)?;

    let flags = FlagBoolean as i16
        | FlagPhrase as i16
        | FlagLovehate as i16
        | FlagBooleanAnyCase as i16
        | FlagWildcard as i16
        | FlagPureNot as i16
        | FlagPartial as i16
        | FlagSpellingCorrection as i16;

    // Combine queries
    //let mut query = qp
    //    .parse_query("a*", flags)
    //    .expect("not found");
    //let mut q = qp
    //    .parse_query_with_prefix("work", flags, "K")
    //    .expect("not found");
    //query = query.add_right(XapianOp::OpAnd, &mut q).expect("not found");

    // Negate a tag
    let mut query = qp
        .parse_query_with_prefix("NOT work", flags, "K")
        .expect("not found");

    let mut enq = db.new_enquire()?;
    enq.set_query(&mut query)?;
    let mut mset = enq.get_mset(0, 100)?;
    let appx_matches = mset.get_matches_estimated()?;
    println!("Approximate Matches {}", appx_matches);

    let mut v = mset.iterator().unwrap();
    while v.is_next().unwrap() {
        let res = v.get_document_data();
        if let Ok(data) = res {
            let v: TikaDocument = serde_json::from_str(&data)?;
            println!("Match {}", v.filename);
        } else {
            eprintln!("No Matches");
        }
        v.next()?;
    }

    Ok(())
}

// TODO Move as much of this as possible out into tui_app.rs
use std::io;
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

/// Interactive query interface
fn interactive_query() -> Result<(), Report> {
    let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;
    let mut qp = QueryParser::new()?;
    let mut stem = Stem::new("en")?;
    qp.set_stemmer(&mut stem)?;

    let flags = FlagBoolean as i16
        | FlagPhrase as i16
        | FlagLovehate as i16
        | FlagBooleanAnyCase as i16
        | FlagWildcard as i16
        | FlagPureNot as i16
        | FlagPartial as i16
        | FlagSpellingCorrection as i16;

    let mut selected: Vec<String> = Vec::new();

    //let mut terminal = tui_app::NewTerminal()?;
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup event handlers
    let events = Events::new();

    // Create default app state
    let mut app = tui_app::TerminalApp::default();

    loop {
        // Draw UI
        terminal.draw(|f| {
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
                    selected = app.get_selected();
                    println!("DONE");
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

            let mut query = qp.parse_query(&app.input, flags).expect("not found");
            let mut enq = db.new_enquire()?;
            enq.set_query(&mut query)?;
            let mut mset = enq.get_mset(0, 100)?;

            app.matches = Vec::new();
            let mut v = mset.iterator().unwrap();
            while v.is_next().unwrap() {
                let res = v.get_document_data();
                if let Ok(data) = res {
                    let v: TikaDocument = serde_json::from_str(&data)?;
                    app.matches.push(v);
                } else {
                    eprintln!("No Matches");
                }
                v.next()?;
            }
        }
    }

    for sel in selected {
        println!("{}", sel);
    }

    Ok(())
}
