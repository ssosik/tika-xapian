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
        //                index(&mut db, &mut tg, &tikadoc)?;
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
                        index(&mut db, &mut tg, &tikadoc)?;
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

    query()?;

    Ok(())
}

fn index(
    db: &mut WritableDatabase,
    tg: &mut TermGenerator,
    tikadoc: &TikaDocument,
) -> Result<(), Report> {
    // Create a new Xapian Document to store attributes on the passed-in TikaDocument
    let mut xdoc = Document::new()?;
    tg.set_document(&mut xdoc)?;

    tg.index_text_with_prefix(&tikadoc.author, "A")?;
    tg.index_text_with_prefix(&tikadoc.date_str()?, "D")?;
    tg.index_text_with_prefix(&tikadoc.filename, "F")?;
    tg.index_text_with_prefix(&tikadoc.full_path.clone().into_string().unwrap(), "F")?;
    tg.index_text_with_prefix(&tikadoc.title, "S")?;
    //tg.index_text_with_prefix(&tikadoc.subtitle, "XS")?;

    xdoc.set_data(&tikadoc.body)?;

    let id = "Q".to_owned() + &tikadoc.filename;
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
    //let flags = FlagDefault as i16;
    //let mut query = qp.parse_query("openssl", flags).expect("not found");
    //let mut query = qp.parse_query("author:ssosik", flags).expect("not found");
    let mut query = qp.parse_query("*", flags).expect("not found");
    let mut enq = db.new_enquire()?;
    enq.set_query(&mut query)?;
    let mut mset = enq.get_mset(0, 10)?;
    let appxMatches = mset.get_matches_estimated()?;
    println!("Approximate Matches {}", appxMatches);

    for mut v in mset.iterator() {
        let res = v.get_document_data();
        if let Ok(data) = res {
            println!("Match {}", data);
        } else {
            eprintln!("No Matches");
        }
    }

    Ok(())
}
