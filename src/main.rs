use anyhow::Result;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagDefault, FlagLovehate, FlagPhrase, FlagSpellingCorrection,
};
#[allow(unused_imports)]
use xapian_rusty::{
    Database, Document, Query, QueryParser, Stem, TermGenerator, WritableDatabase, BRASS,
    DB_CREATE_OR_OPEN, DB_CREATE_OR_OVERWRITE,
};


fn index() -> Result<()> {
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

    db.commit()?;

    Ok(())
}

#[allow(unused_variables, non_snake_case)]
fn query() -> Result<()> {
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

    // How to get results?
    let it = mset.iterator();
    match it {
        Ok(mut s) => {
            // The call here causes compilation failure
            println!("Match {:?}", s.get_document_data());
            println!("Match");
        }
        Err(e) => {
            eprintln!("No Matched");
        }
    };
    // This also doesn't work in the same way
    //for mut v in mset.iterator() {
    //    let data = v.get_document_data()?;
    //    println!("Match {}", data);
    //}

    Ok(())
}

fn main() -> Result<()> {
    index()?;
    query()?;

    Ok(())
}
