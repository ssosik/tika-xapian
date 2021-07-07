use anyhow::Result;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagLovehate, FlagPhrase, FlagSpellingCorrection,
};
use xapian_rusty::{
    Database, Document, Query, QueryParser, Stem, TermGenerator, WritableDatabase, BRASS,
    DB_CREATE_OR_OPEN, DB_CREATE_OR_OVERWRITE,
};

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
    doc.set_data("data foo bar thing")?;
    doc.add_boolean_term("my-id-term")?;
    db.replace_document("my-id-term", &mut doc)?;

    let mut doc = Document::new()?;
    tg.set_document(&mut doc)?;
    tg.index_text("two bar thing")?;
    doc.set_data("data two bar thing")?;
    doc.add_boolean_term("my-id-term2")?;
    db.replace_document("my-id-term2", &mut doc)?;

    println!("Hello, world!");

    let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;
    let mut qp = QueryParser::new()?;
    qp.set_stemmer(&mut stem)?;
    let flags = FlagBoolean as i16
        | FlagPhrase as i16
        | FlagLovehate as i16
        | FlagBooleanAnyCase as i16
        | FlagSpellingCorrection as i16;
    let mut query = qp.parse_query("foo and thing", flags)?;
    let mut enq = db.new_enquire()?;
    enq.set_query(&mut query)?;
    let mut mset = enq.get_mset(0,10)?;
    //for mut v in mset.iterator().into_iter() {
    //    println!("Match {}", v.get_document_data()?)
    //}

    Ok(())
}
