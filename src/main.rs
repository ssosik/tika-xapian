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
#[allow(unused_imports)]
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

    //let q = parse_user_query(r#"aaabcde c AND NOT vkms"#)?;
    //let q = parse_user_query(r#"foobar AND NOT vkms"#)?;
    //let q = parse_user_query(r#"foobar AND vkms"#)?;
    //let q = parse_user_query(r#"openssl x509 and not vkms and not curl"#)?;
    //let q = parse_user_query(r#""#)?;
    //perform_query(q)?;

    interactive_query()?;

    Ok(())
}

#[allow(unused_imports)]
use nom::{
    bytes::complete::{is_not, tag_no_case, take_while1, take_while_m_n},
    character::complete::{alpha1, alphanumeric1, anychar, char, space0},
    combinator::{map_res, value},
    error::{ErrorKind, ParseError},
    sequence::{terminated, tuple},
    Err,
    {
        add_return_error, alt, call, char, complete, delimited, error_node_position,
        error_position, escaped, is_not, named, none_of, one_of, peek, tag, take_until, take_while,
        tuple,
    },
};

#[allow(unused_imports)]
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag},
    character::complete::none_of,
    sequence::delimited,
    IResult,
};
use std::str;

named!(
    doublequoted,
    delimited!(tag!(r#"""#), is_not(r#"""#), tag!(r#"""#))
);

// Xapian tags in human format, e.g. "author;" or "title:"
#[derive(Debug)]
pub enum XTag {
    Author,
    Date,
    Filename,
    Fullpath,
    Title,
    Subtitle,
    Tag,
}

impl XTag {
    fn to_xapian<'a>(self) -> &'a [u8] {
        match self {
            XTag::Author => "A".as_bytes(),
            XTag::Date => "D".as_bytes(),
            XTag::Filename => "F".as_bytes(),
            XTag::Fullpath => "F".as_bytes(),
            XTag::Title => "S".as_bytes(),
            XTag::Subtitle => "XS".as_bytes(),
            XTag::Tag => "K".as_bytes(),
        }
    }
}

pub fn match_xtag(input: &str) -> IResult<&str, &XTag> {
    alt((
        value(&XTag::Author, tag("author:")),
        value(&XTag::Date, tag("date:")),
        value(&XTag::Filename, tag("filename:")),
        value(&XTag::Fullpath, tag("fullpath:")),
        value(&XTag::Title, tag("title:")),
        value(&XTag::Subtitle, tag("subtitle:")),
        value(&XTag::Tag, tag("tag:")),
    ))(input)
}

pub fn match_op(input: &str) -> IResult<&str, &XapianOp> {
    // Note 1:
    // From https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md
    // Note that case insensitive comparison is not well defined for unicode,
    // and that you might have bad surprises
    // Note 2:
    // Order these by longest match, according to
    // https://docs.rs/nom/6.2.1/nom/macro.alt.html#behaviour-of-alt
    alt((
        value(&XapianOp::OpAndNot, tag_no_case("AND NOT")),
        value(&XapianOp::OpAnd, tag_no_case("AND")),
        value(&XapianOp::OpXor, tag_no_case("XOR")),
        value(&XapianOp::OpOr, tag_no_case("OR")),
        // OpAndMaybe,
        // OpFilter,
        // OpNear,
        // OpPhrase,
        // OpValueRange,
        // OpScaleWeight,
        // OpEliteSet,
        // OpValueGe,
        // OpValueLe,
        // OpSynonym,
    ))(input)
}

// TODO is there a better way to handle case insensitity here?
named!(
    take_up_to_operator,
    alt!(
        complete!(take_until!("AND NOT"))
            | complete!(take_until!("and not"))
            | complete!(take_until!("AND"))
            | complete!(take_until!("and"))
            | complete!(take_until!("XOR"))
            | complete!(take_until!("xor"))
            | complete!(take_until!("OR"))
            | complete!(take_until!("or"))
    )
);

fn parse_user_query(mut qstr: &str) -> Result<Query, Report> {
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

    // Accumulators, start them off as empty options
    let mut query: Option<Query> = None;
    let mut operator: Option<&XapianOp> = None;

    while qstr.len() > 0 {
        //println!("Processing '{}'", qstr);

        match take_up_to_operator(qstr.as_bytes()) {
            Err(e) => {
                //eprintln!("Take up to operator error: '{}' in: '{}'", e, qstr);
                //println!("Break Query: '{}' {}", qstr, e);
                //break;

                // TODO reduce duplication here, test that 'e' is expected Error
                if query.is_none() {
                    let q = qp
                        .parse_query(qstr, flags)
                        .expect("No more operators: QueryParser error");
                    //println!("parsed query string '{}'", qstr);
                    query = Some(q);
                } else {
                    let op = match operator {
                        Some(&XapianOp::OpAndNot) => {
                            //println!("No more operators: Use Operator And Not");
                            XapianOp::OpAndNot
                        }
                        Some(&XapianOp::OpAnd) => {
                            //println!("No more operators: Use Operator And");
                            XapianOp::OpAnd
                        }
                        Some(&XapianOp::OpXor) => {
                            //println!("No more operators: Use Operator Xor");
                            XapianOp::OpXor
                        }
                        Some(&XapianOp::OpOr) => {
                            //println!("No more operators: Use Operator Or");
                            XapianOp::OpOr
                        }
                        _ => {
                            //eprintln!("No more operators: Found unsupported Xapian Operation");
                            XapianOp::OpAnd
                        }
                    };

                    //println!("No more operators: appended query string {}", qstr);
                    query = Some(
                        query
                            .unwrap()
                            .add_right(op, &mut qp.parse_query(qstr, flags)?)
                            .expect("No more operators: Failed to add_right()"),
                    );
                }
            }
            Ok((remaining, current)) => {
                let curr_query = str::from_utf8(&current)?;
                //println!("Took Query up to operator: '{}'", curr_query);
                qstr = str::from_utf8(&remaining)?;
                if query.is_none() {
                    let q = qp
                        .parse_query(curr_query, flags)
                        .expect("QueryParser error");
                    //println!("parsed query string '{}'", curr_query);
                    query = Some(q);
                } else {
                    let op = match operator {
                        Some(&XapianOp::OpAndNot) => {
                            //println!("Use Operator And Not");
                            XapianOp::OpAndNot
                        }
                        Some(&XapianOp::OpAnd) => {
                            //println!("Use Operator And");
                            XapianOp::OpAnd
                        }
                        Some(&XapianOp::OpXor) => {
                            //println!("Use Operator Xor");
                            XapianOp::OpXor
                        }
                        Some(&XapianOp::OpOr) => {
                            //println!("Use Operator Or");
                            XapianOp::OpOr
                        }
                        _ => {
                            eprintln!("Found unsupported Xapian Operation");
                            XapianOp::OpAnd
                        }
                    };

                    //println!("appended query string {}", curr_query);
                    query = Some(
                        query
                            .unwrap()
                            .add_right(op, &mut qp.parse_query(curr_query, flags)?)
                            .expect("Failed to add_right()"),
                    );
                }
            }
        };

        //println!("MATCH OP: {}", qstr);
        match match_op(&qstr) {
            Ok((remaining, op)) => {
                operator = match op {
                    XapianOp::OpAndNot => {
                        //println!("Set Operator And Not");
                        Some(&XapianOp::OpAndNot)
                    }
                    XapianOp::OpAnd => {
                        //println!("Set Operator And");
                        Some(&XapianOp::OpAnd)
                    }
                    XapianOp::OpXor => {
                        //println!("Set Operator Xor");
                        Some(&XapianOp::OpXor)
                    }
                    XapianOp::OpOr => {
                        //println!("Set Operator Or");
                        Some(&XapianOp::OpOr)
                    }
                    _ => {
                        //eprintln!("Found unsupported Xapian Operation");
                        Some(&XapianOp::OpAnd)
                    }
                };
                qstr = remaining
            }
            Err(e) => {
                //eprintln!("Match Op error: '{}' in '{}'", e, qstr);
                break;
            }
        };
    }

    //let dblqtd = r#""openssl x509" AND vkms"#;
    //match doublequoted(dblqtd.as_bytes()) {
    //    Ok((a, b)) => {
    //        println!(
    //            "DBL A: {} B:{}",
    //            str::from_utf8(a).unwrap(),
    //            str::from_utf8(b).unwrap()
    //        );
    //    }
    //    Err(e) => {
    //        println!("DoubleQuote no good: {}", e);
    //    }
    //};

    //let qstr1 = r#"openssl AND NOT author:"steve sosik""#;
    //match doublequoted(qstr1.as_bytes()) {
    //    Ok((a, b)) => {
    //        println!(
    //            "THING A: {} B:{}",
    //            str::from_utf8(a).unwrap(),
    //            str::from_utf8(b).unwrap()
    //        );
    //    }
    //    Err(e) => {
    //        println!("Thing no good: {}", e);
    //    }
    //};

    //println!("Done");
    Ok(query.unwrap())
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

fn perform_query(mut q: Query) -> Result<(), Report> {
    let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;

    let mut enq = db.new_enquire()?;
    enq.set_query(&mut q)?;
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

#[allow(dead_code)]
fn perform_query_canned() -> Result<(), Report> {
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
#[allow(unused_imports)]
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

/// Interactive query interface
#[allow(dead_code)]
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
                    //println!("DONE");
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
            let mut query = parse_user_query(&app.input)?;

            let mut enq = db.new_enquire()?;
            enq.set_query(&mut query)?;
            let mut mset = enq.get_mset(0, 100)?;

            // TODO: extract the following code into one place
            // perform_query(q)?;
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
