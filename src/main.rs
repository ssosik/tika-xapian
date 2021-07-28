mod util;
mod tika_core;
mod tui_app;

use color_eyre::Report;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagDefault, FlagLovehate, FlagPhrase, FlagSpellingCorrection,
};

use xapian_rusty::{
    Database, Document, Query, QueryParser, Stem, TermGenerator, WritableDatabase, BRASS,
    DB_CREATE_OR_OPEN, DB_CREATE_OR_OVERWRITE,
};

use chrono::{DateTime, FixedOffset};
use clap::{App, Arg, ArgMatches, SubCommand};
use glob::{glob, Paths};
use toml::Value as tomlVal;
use yaml_rust::YamlEmitter;

use crate::util::event::{Event, Events};
//use unicode_width::UnicodeWidthStr;

/// Example FrontMatter + Markdown doc to index:
///
/// ---
/// author: Steve Sosik
/// date: 2021-06-22T12:48:16-0400
/// tags:
/// - tika
/// title: This is an example note
/// ---
///
/// Some note here formatted with Markdown syntax
///

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
    setup(&default_config_file)?;

    index()?;
    query()?;

    Ok(())
}

fn index() -> Result<(), Report> {
    let mut db = WritableDatabase::new("mydb", BRASS, DB_CREATE_OR_OPEN)?;
    let mut tg = TermGenerator::new()?;
    let mut stem = Stem::new("en")?;
    tg.set_stemmer(&mut stem)?;

    let mut doc = Document::new()?;
    tg.set_document(&mut doc)?;
    tg.index_text("foo bar thing")?;
    doc.set_data("data foo bar thing")?;
    doc.add_boolean_term("my-id-term")?;
    db.replace_document("my-id-term", &mut doc)?;

    let mut doc = Document::new()?;
    tg.set_document(&mut doc)?;
    tg.index_text("two bar thing")?;
    doc.set_data("data two bar thing")?;
    doc.add_boolean_term("my-id-term2")?;
    db.replace_document("my-id-term2", &mut doc)?;

    db.commit()?;

    Ok(())
}

#[allow(unused_variables, non_snake_case)]
fn query() -> Result<(), Report> {
    let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;
    let mut qp = QueryParser::new()?;
    let mut stem = Stem::new("en")?;
    qp.set_stemmer(&mut stem)?;
    let flags = FlagBoolean as i16
        | FlagPhrase as i16
        | FlagLovehate as i16
        | FlagBooleanAnyCase as i16
        | FlagSpellingCorrection as i16;
    let flags = FlagDefault as i16;
    //let mut query = qp.parse_query("foo", flags).expect("not found");
    let mut query = qp.parse_query("bar", flags).expect("not found");
    //let mut query = qp.parse_query("NOT foo", flags).expect("not found");
    //let mut query = qp.parse_query("foo AND thing", flags).expect("not found");
    let mut enq = db.new_enquire()?;
    enq.set_query(&mut query)?;
    let mut mset = enq.get_mset(0, 10)?;
    let appxMatches = mset.get_matches_estimated()?;
    println!("Approximate Matches {}", appxMatches);

    for mut v in mset.iterator() {
        let data = v.get_document_data()?;
        println!("Match {}", data);
    }

    Ok(())
}
