mod tika_document;
mod tui_app;
mod util;

use crate::tika_document::{parse_file, TikaDocument};
use crate::util::event::{Event, Events};
use crate::util::glob_files;
use clap::{App, Arg, ArgMatches, SubCommand};
use color_eyre::Report;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagDefault, FlagLovehate, FlagPhrase, FlagSpellingCorrection,
};
use xapian_rusty::{
    Database, Document, Query, QueryParser, Stem, TermGenerator, WritableDatabase, BRASS,
    DB_CREATE_OR_OPEN, DB_CREATE_OR_OVERWRITE,
};

//use unicode_width::UnicodeWidthStr;

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

        for entry in glob_files(
            &cli.value_of("config").unwrap(),
            cli.value_of("source"),
            cli.occurrences_of("v") as i8,
        )
        .expect("Failed to read glob pattern")
        {
            match entry {
                Ok(path) => {
                    if let Ok(doc) = parse_file(&path) {
                        if let Some(f) = path.to_str() {
                            index(&mut db, &mut tg, &doc)?;
                            if cli.occurrences_of("v") > 0 {
                                println!("✅ {}", f);
                            }
                        } else {
                            eprintln!(
                                "❌ Failed to parse time '{}' from {}",
                                doc.date, doc.filename
                            );
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

    query()?;

    Ok(())
}

fn index(
    db: &mut WritableDatabase,
    tg: &mut TermGenerator,
    doc: &TikaDocument,
) -> Result<(), Report> {
    let mut xdoc = Document::new()?;
    tg.set_document(&mut xdoc)?;

    tg.index_text_with_prefix(&doc.author, "A")?;
    tg.index_text_with_prefix(&doc.date_str()?, "D")?;
    tg.index_text_with_prefix(&doc.filename, "F")?;
    tg.index_text_with_prefix(&doc.title, "S")?;
    //tg.index_text_with_prefix(&doc.subtitle, "XS")?;

    xdoc.set_data(&doc.body)?;

    let id = "Q".to_owned() + &doc.filename;
    xdoc.add_boolean_term(&id)?;
    db.replace_document(&id, &mut xdoc)?;

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
    let mut query = qp.parse_query("openssl", flags).expect("not found");
    //let mut query = qp.parse_query("NOT foo", flags).expect("not found");
    //let mut query = qp.parse_query("foo AND thing", flags).expect("not found");
    let mut enq = db.new_enquire()?;
    enq.set_query(&mut query)?;
    let mut mset = enq.get_mset(0, 10)?;
    let appxMatches = mset.get_matches_estimated()?;
    println!("Approximate Matches {}", appxMatches);

    for mut v in mset.iterator() {
        //let data = v.get_document_data()?;
        //println!("Match {}", data);
        let res = v.get_document_data();
        if let Ok(data) = res {
            println!("Match {}", data);
        } else {
            eprintln!("No Matches");
        }
    }

    Ok(())
}
