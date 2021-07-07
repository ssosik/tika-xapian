use anyhow::Result;
use xapian_rusty::{BRASS, Database, Document, DB_CREATE_OR_OPEN, DB_CREATE_OR_OVERWRITE, TermGenerator, Stem, Query, QueryParser, WritableDatabase};

// export CARGO_MANIFEST_DIR=/Users/ssosik/workspace/xapian-rusty
// export CARGO_TARGET_DIR=target/foo
// cargo run

fn main() -> Result<()> {
    let mut db = WritableDatabase::new("mydb", BRASS, DB_CREATE_OR_OVERWRITE)?;
    //let mut db = WritableDatabase::new("mydb", BRASS, DB_CREATE_OR_OPEN)?;
    let mut tg = TermGenerator::new()?;
    let mut stem = Stem::new("en")?;
    tg.set_stemmer(&mut stem)?;

    let mut doc = Document::new()?;
    tg.set_document(&mut doc)?;
    tg.index_text("foo bar thing")?;
    println!("Hello, world!");

    doc.set_data("data foo bar thing")?;
    doc.add_boolean_term("my-id-term")?;
    db.replace_document("my-id-term", &mut doc)?;

    Ok(())
}
