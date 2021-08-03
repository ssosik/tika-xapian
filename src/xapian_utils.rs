use crate::tika_document::TikaDocument;
use color_eyre::Report;
use nom::{
    bytes::complete::{is_not, tag, tag_no_case},
    combinator::value,
    {alt, branch::alt, complete, delimited, named, tag, take_until, IResult},
};
use std::str;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagLovehate, FlagPartial, FlagPhrase, FlagPureNot,
    FlagSpellingCorrection, FlagWildcard,
};
use xapian_rusty::{Database, Query, QueryParser, Stem, XapianOp, DB_CREATE_OR_OVERWRITE};

// Xapian tags in human format, e.g. "author;" or "title:"
#[derive(Debug, Clone, Copy)]
pub enum XapianTag {
    Author,
    Date,
    Filename,
    Fullpath,
    Title,
    Subtitle,
    Tag,
}

impl XapianTag {
    fn to_xapian<'a>(self) -> &'a str {
        match self {
            XapianTag::Author => "A",
            XapianTag::Date => "D",
            XapianTag::Filename => "F",
            XapianTag::Fullpath => "F",
            XapianTag::Title => "S",
            XapianTag::Subtitle => "XS",
            XapianTag::Tag => "K",
        }
    }
}

pub fn match_xapiantag(input: &str) -> IResult<&str, &XapianTag> {
    alt((
        value(&XapianTag::Author, tag("author:")),
        value(&XapianTag::Date, tag("date:")),
        value(&XapianTag::Filename, tag("filename:")),
        value(&XapianTag::Fullpath, tag("fullpath:")),
        value(&XapianTag::Title, tag("title:")),
        value(&XapianTag::Subtitle, tag("subtitle:")),
        value(&XapianTag::Tag, tag("tag:")),
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

//struct QueryParseState {
//    query: Option<Query>,
//    operator: Option<i32>,
//}
//
//impl QueryParseState {
//    fn new<'a>() -> &'a QueryParseState {
//        &QueryParseState {
//            query: None,
//            operator: None,
//        }
//    }
//
//    fn operator(self) -> XapianOp {
//        return match self.operator {
//            x if x.unwrap() == XapianOp::OpAnd as i32 => XapianOp::OpAnd,
//            x if x.unwrap() == XapianOp::OpAndNot as i32 => XapianOp::OpAndNot,
//            x if x.unwrap() == XapianOp::OpXor as i32 => XapianOp::OpXor,
//            x if x.unwrap() == XapianOp::OpOr as i32 => XapianOp::OpOr,
//            _ => unreachable!(),
//        };
//    }
//
//    fn update_query<'a>(
//        mut self,
//        mut qp: QueryParser,
//        flags: i16,
//        qstr: &str,
//    ) -> Result<QueryParseState, Report> {
//        if self.query.is_none() {
//            self.query = Some(
//                qp.parse_query(qstr, flags)
//                    .expect("No more operators: QueryParser error"),
//            );
//        } else {
//            self.query = Some(
//                self.query
//                    .unwrap()
//                    .add_right(self.operator(), &mut qp.parse_query(qstr, flags)?)
//                    .expect("No more operators: Failed to add_right()"),
//            );
//        }
//        return Ok(self);
//    }
//
//    fn update_operator<'a>(
//        mut self,
//        qp: QueryParser,
//        operator: XapianOp,
//    ) -> Result<QueryParseState, Report> {
//        self.operator = match operator {
//            XapianOp::OpAndNot => {
//                //println!("No more operators: Use Operator And Not");
//                Some(XapianOp::OpAndNot as i32)
//            }
//            XapianOp::OpAnd => {
//                //println!("No more operators: Use Operator And");
//                Some(XapianOp::OpAnd as i32)
//            }
//            XapianOp::OpXor => {
//                //println!("No more operators: Use Operator Xor");
//                Some(XapianOp::OpXor as i32)
//            }
//            XapianOp::OpOr => {
//                //println!("No more operators: Use Operator Or");
//                Some(XapianOp::OpOr as i32)
//            }
//            _ => {
//                panic!("No more operators: Found unsupported Xapian Operation");
//            }
//        };
//        return Ok(self);
//    }
//}

use nom::is_not;
use nom::recognize;
use nom::tuple;

named!(
    doublequoted,
    recognize!(delimited!(tag!(r#"""#), is_not(r#"""#), tag!(r#"""#)))
);

named!(
    tagdoublequoted,
    recognize!(tuple!(
        is_not!(r#" :""#),
        tag!(r#":"#),
        tag!(r#"""#),
        is_not!(r#"""#),
        tag!(r#"""#)
    ))
);

named!(
    tagword,
    recognize!(tuple!(is_not!(r#" :""#), tag!(r#":"#), is_not!(r#" "#)))
);

named!(
    operator_expr,
    recognize!(alt!(
        complete!(take_until!("AND NOT"))
            | complete!(take_until!("and not"))
            | complete!(take_until!("AND"))
            | complete!(take_until!("and"))
            | complete!(take_until!("XOR"))
            | complete!(take_until!("xor"))
            | complete!(take_until!("OR"))
            | complete!(take_until!("or"))
    ))
);

pub fn test_user_query(mut qstr: &str) -> Result<(), Report> {
    if let Ok((a, b)) = doublequoted(qstr.as_bytes()) {
        let a = str::from_utf8(a).unwrap();
        let b = str::from_utf8(b).unwrap();
        println!("DoubleQuoted a:'{}' b:'{}'", b, a);
    } else if let Ok((a, b)) = tagdoublequoted(qstr.as_bytes()) {
        let a = str::from_utf8(a).unwrap();
        let b = str::from_utf8(b).unwrap();
        println!("TagDoubleQuoted a:'{}' b:'{}'", a, b);
        if let Ok((s, tag)) = match_xapiantag(b) {
            println!("Tag: {} {}", tag.to_xapian(), s);
        } else {
            println!("NoTag");
        }
    } else if let Ok((a, b)) = operator_expr(qstr.as_bytes()) {
        let a = str::from_utf8(a).unwrap();
        let b = str::from_utf8(b).unwrap();
        println!("Operator a:'{}' b:'{}'", b, a);
    } else if let Ok((a, b)) = tagword(qstr.as_bytes()) {
        let a = str::from_utf8(a).unwrap();
        let b = str::from_utf8(b).unwrap();
        println!("TagWord a:'{}' b:'{}'", b, a);
        if let Ok((s, tag)) = match_xapiantag(b) {
            println!("Tag: {} {}", tag.to_xapian(), s);
        } else {
            println!("NoTag");
        }
    } else {
        println!("Bare expr:'{}'", qstr);
    };

    Ok(())
}

pub fn parse_user_query(mut qstr: &str) -> Result<Query, Report> {
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
    //let mut accumulator = QueryParseState::new();

    while qstr.len() > 0 {
        //println!("Processing '{}'", qstr);

        match take_up_to_operator(qstr.as_bytes()) {
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
            Err(_) => {
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
            Err(_) => {
                //eprintln!("Match Op error: '{}' in '{}'", e, qstr);
                break;
            }
        };
    }

    //named!(
    //    doublequoted,
    //    delimited!(tag!(r#"""#), is_not(r#"""#), tag!(r#"""#))
    //);

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

    match query {
        Some(ret) => Ok(ret),
        None => Ok(qp.parse_query("", flags).expect("QueryParser error")),
    }
}

//fn query_db(mut db: Database, mut q: Query) -> Result<Vec<TikaDocument>, Report> {
pub fn query_db(mut q: Query) -> Result<Vec<TikaDocument>, Report> {
    // TODO Reuse existing DB instead of creating a new one on each query
    let mut db = Database::new_with_path("mydb", DB_CREATE_OR_OVERWRITE)?;
    let mut enq = db.new_enquire()?;
    enq.set_query(&mut q)?;
    // TODO set this based on terminal height?
    let mut mset = enq.get_mset(0, 100)?;

    // TODO with verbose logging log this:
    //let appx_matches = mset.get_matches_estimated()?;
    //println!("Approximate Matches {}", appx_matches);

    let mut matches = Vec::new();
    let mut v = mset.iterator().unwrap();
    while v.is_next().unwrap() {
        let res = v.get_document_data();
        // Can use flatten() or some other iterators/combinators?
        if let Ok(data) = res {
            let v: TikaDocument = serde_json::from_str(&data)?;
            //println!("Match {}", v.filename);
            matches.push(v);
        }
        v.next()?;
    }

    Ok(matches)
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
