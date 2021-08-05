use crate::tika_document::TikaDocument;
use color_eyre::Report;
#[allow(unused)]
use nom::{
    bytes::streaming::{is_not, tag, tag_no_case},
    character::streaming::{alphanumeric0, alphanumeric1, multispace0, multispace1, space0},
    combinator::{recognize, value},
    multi::{many0, many1},
    sequence::{delimited, separated_pair, tuple},
    {alt, branch::alt, complete, delimited, named, tag, take_until, value}, // {IResult},
};
use std::convert::From;
use std::fmt;
use std::str;
use xapian_rusty::FeatureFlag::{
    FlagBoolean, FlagBooleanAnyCase, FlagLovehate, FlagPartial, FlagPhrase, FlagPureNot,
    FlagSpellingCorrection, FlagWildcard,
};
use xapian_rusty::{Database, Query, QueryParser, Stem, XapianOp, DB_CREATE_OR_OVERWRITE};

// The most helpful write-up on using Nom that I've seen so far:
//   https://iximiuz.com/en/posts/rust-writing-parsers-with-nom/

// Local representation of xapian expression operators, most notably these are Copy!
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatchOp {
    And,
    AndNot,
    Or,
    Xor,
    AndMaybe,
    Filter,
    Near,
    Phrase,
    ValueRange,
    ScaleWeight,
    EliteSet,
    ValueGe,
    ValueLe,
    Synonym,
}

// Support converting into and from XapianOps
impl From<MatchOp> for XapianOp {
    fn from(item: MatchOp) -> Self {
        match item {
            MatchOp::And => XapianOp::OpAnd,
            MatchOp::AndNot => XapianOp::OpAndNot,
            MatchOp::Or => XapianOp::OpOr,
            MatchOp::Xor => XapianOp::OpXor,
            MatchOp::AndMaybe => XapianOp::OpAndMaybe,
            MatchOp::Filter => XapianOp::OpFilter,
            MatchOp::Near => XapianOp::OpNear,
            MatchOp::Phrase => XapianOp::OpPhrase,
            MatchOp::ValueRange => XapianOp::OpValueRange,
            MatchOp::ScaleWeight => XapianOp::OpScaleWeight,
            MatchOp::EliteSet => XapianOp::OpEliteSet,
            MatchOp::ValueGe => XapianOp::OpValueGe,
            MatchOp::ValueLe => XapianOp::OpValueLe,
            MatchOp::Synonym => XapianOp::OpSynonym,
        }
    }
}

impl From<XapianOp> for MatchOp {
    fn from(item: XapianOp) -> Self {
        match item {
            XapianOp::OpAnd => MatchOp::And,
            XapianOp::OpAndNot => MatchOp::AndNot,
            XapianOp::OpOr => MatchOp::Or,
            XapianOp::OpXor => MatchOp::Xor,
            XapianOp::OpAndMaybe => MatchOp::AndMaybe,
            XapianOp::OpFilter => MatchOp::Filter,
            XapianOp::OpNear => MatchOp::Near,
            XapianOp::OpPhrase => MatchOp::Phrase,
            XapianOp::OpValueRange => MatchOp::ValueRange,
            XapianOp::OpScaleWeight => MatchOp::ScaleWeight,
            XapianOp::OpEliteSet => MatchOp::EliteSet,
            XapianOp::OpValueGe => MatchOp::ValueGe,
            XapianOp::OpValueLe => MatchOp::ValueLe,
            XapianOp::OpSynonym => MatchOp::Synonym,
        }
    }
}

impl fmt::Display for MatchOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MatchOp::And => write!(f, "<And>"),
            MatchOp::AndNot => write!(f, "<AndNot>"),
            MatchOp::Or => write!(f, "<Or>"),
            MatchOp::Xor => write!(f, "<Xor>"),
            MatchOp::AndMaybe => write!(f, "<AndMaybe>"),
            MatchOp::Filter => write!(f, "<Filter>"),
            MatchOp::Near => write!(f, "<Near>"),
            MatchOp::Phrase => write!(f, "<Phrase>"),
            MatchOp::ValueRange => write!(f, "<ValueRange>"),
            MatchOp::ScaleWeight => write!(f, "<ScaleWeight>"),
            MatchOp::EliteSet => write!(f, "<EliteSet>"),
            MatchOp::ValueGe => write!(f, "<ValueGe>"),
            MatchOp::ValueLe => write!(f, "<ValueLe>"),
            MatchOp::Synonym => write!(f, "<Synonym>"),
        }
    }
}

pub fn matchop(input: &str) -> IResult<MatchOp> {
    alt((
        value(MatchOp::AndNot, tag_no_case("AND NOT")),
        value(MatchOp::And, tag_no_case("AND")),
        value(MatchOp::Xor, tag_no_case("XOR")),
        value(MatchOp::Or, tag_no_case("OR")),
        value(MatchOp::AndMaybe, tag_no_case("AND MAYBE")),
        value(MatchOp::Filter, tag_no_case("FILTER")),
        value(MatchOp::Near, tag_no_case("NEAR")),
        value(MatchOp::Phrase, tag_no_case("PHRASE")),
        value(MatchOp::ValueRange, tag_no_case("RANGE")),
        value(MatchOp::ScaleWeight, tag_no_case("SCALED")),
        value(MatchOp::EliteSet, tag_no_case("ELITE")),
        value(MatchOp::ValueGe, tag_no_case(">")),
        value(MatchOp::ValueLe, tag_no_case("<")),
        value(MatchOp::Synonym, tag_no_case("SYNONYM")),
    ))(Span::new(input))
}

use nom_locate::LocatedSpan;

pub type Span<'a> = LocatedSpan<&'a str>;

pub type IResult<'a, O> = nom::IResult<Span<'a>, O>;

#[allow(dead_code)]
fn word(input: Span) -> IResult<Span> {
    // TODO should more characters be supported in a "word"?
    // Use `recognize` here to discard the actual parsed value and return the matched substring as
    // a result
    recognize(many1(alt((recognize(alphanumeric1), recognize(tag("_"))))))(input)
}

struct ExpectedParseResult<'a> {
    matched_fragment: &'a str,
    matched_offset: usize,
    matched_line: u32,
    matched_column: usize,
    remainder_fragment: &'a str,
    remainder_offset: usize,
    remainder_line: u32,
    remainder_column: usize,
}

impl ExpectedParseResult<'_> {
    fn new<'a>(
        mf: &'a str,
        mo: usize,
        ml: u32,
        mc: usize,
        rf: &'a str,
        ro: usize,
        rl: u32,
        rc: usize,
    ) -> ExpectedParseResult<'a> {
        ExpectedParseResult {
            matched_fragment: mf,
            matched_offset: mo,
            matched_line: ml,
            matched_column: mc,
            remainder_fragment: rf,
            remainder_offset: ro,
            remainder_line: rl,
            remainder_column: rc,
        }
    }
    fn compare(self, f: &dyn Fn(Span) -> IResult<Span>, s: &str) {
        let (remainder, matched) = f(Span::new(s)).expect("Failed to parse input");

        assert_eq!(&self.matched_fragment, matched.fragment());
        assert_eq!(self.matched_offset, matched.location_offset());
        assert_eq!(self.matched_line, matched.location_line());
        assert_eq!(self.matched_column, matched.get_column());

        assert_eq!(&self.remainder_fragment, remainder.fragment());
        assert_eq!(self.remainder_offset, remainder.location_offset());
        assert_eq!(self.remainder_line, remainder.location_line());
        assert_eq!(self.remainder_column, remainder.get_column());
    }
}

#[cfg(test)]
mod word_tests {
    use super::*;
    #[test]
    fn one_word_no_trailing_space() {
        assert!(word(Span::new(r#"foo"#)).is_err())
    }

    #[test]
    fn one_word_with_trailing_space() {
        ExpectedParseResult::new(&"foo", 0, 1, 1, &" ", 3, 1, 4).compare(&word, &r#"foo "#)
    }

    #[test]
    fn one_word_with_trailing_newline() {
        ExpectedParseResult::new(&"foo", 0, 1, 1, &"\\n", 3, 1, 4).compare(&word, &r#"foo\n"#)
    }

    #[test]
    fn two_space_separated_words() {
        ExpectedParseResult::new(&"foo", 0, 1, 1, &" bar", 3, 1, 4).compare(&word, &r#"foo bar"#)
    }
}

fn words(input: Span) -> IResult<Span> {
    recognize(many1(alt((recognize(multispace1), recognize(word)))))(input)
}

#[cfg(test)]
mod words_tests {
    use super::*;
    #[test]
    fn one_word_no_trailing_newline() {
        assert!(words(Span::new(r#"foo"#)).is_err())
    }

    #[test]
    fn one_word() {
        ExpectedParseResult::new(&"foo", 0, 1, 1, &"\\n", 3, 1, 4).compare(&words, &r#"foo\n"#)
    }

    #[test]
    fn two_space_separated_words() {
        ExpectedParseResult::new(&"foo bar", 0, 1, 1, &"\\n", 7, 1, 8)
            .compare(&words, &r#"foo bar\n"#)
    }
}

fn quoted(input: Span) -> IResult<Span> {
    recognize(alt((
        delimited(
            recognize(tag(r#"""#)),
            recognize(words),
            recognize(tag(r#"""#)),
        ),
        delimited(
            recognize(tag(r#"'"#)),
            recognize(words),
            recognize(tag(r#"'"#)),
        ),
    )))(input)
}

#[cfg(test)]
mod quoted_tests {
    use super::*;
    #[test]
    fn one_word_no_trailing_space() {
        ExpectedParseResult::new(&"\"foo\"", 0, 1, 1, &"", 5, 1, 6).compare(&quoted, &r#""foo""#)
    }

    #[test]
    fn one_word_with_trailing_space() {
        ExpectedParseResult::new(&"\"foo \"", 0, 1, 1, &"", 6, 1, 7).compare(&quoted, &r#""foo ""#)
    }

    #[test]
    fn two_words() {
        ExpectedParseResult::new(&"\"foo bar\"", 0, 1, 1, &"", 9, 1, 10)
            .compare(&quoted, &r#""foo bar""#)
    }

    #[test]
    fn single_quote_one_word_no_trailing_space() {
        ExpectedParseResult::new(&"\'foo\'", 0, 1, 1, &"", 5, 1, 6).compare(&quoted, &r#"'foo'"#)
    }

    #[test]
    fn single_quote_one_word_with_trailing_space() {
        ExpectedParseResult::new(&"\'foo \'", 0, 1, 1, &"", 6, 1, 7).compare(&quoted, &r#"'foo '"#)
    }

    #[test]
    fn single_quote_two_words() {
        ExpectedParseResult::new(&"\'foo bar\'", 0, 1, 1, &"", 9, 1, 10)
            .compare(&quoted, &r#"'foo bar'"#)
    }

    #[test]
    fn tag_entirely_single_quoted() {
        // The colon character currently isn't an allowed `word` character
        assert!(tagged(Span::new(r#"'foo:bar'"#)).is_err())
    }

    #[test]
    fn tag_entirely_double_quoted() {
        // The colon character currently isn't an allowed `word` character
        assert!(tagged(Span::new(r#""foo:bar""#)).is_err())
    }
}

fn tagged(input: Span) -> IResult<Span> {
    recognize(separated_pair(word, tag(":"), alt((quoted, word))))(input)
}

#[cfg(test)]
mod tagged_tests {
    use super::*;
    #[test]
    fn one_word_no_trailing_space() {
        assert!(tagged(Span::new(r#"foo:bar"#)).is_err())
    }

    #[test]
    fn one_word_with_trailing_space() {
        ExpectedParseResult::new(&"foo:bar", 0, 1, 1, &" ", 7, 1, 8)
            .compare(&tagged, &r#"foo:bar "#)
    }

    #[test]
    fn two_words() {
        ExpectedParseResult::new(&"tag:foo", 0, 1, 1, &" bar", 7, 1, 8)
            .compare(&tagged, &r#"tag:foo bar"#)
    }

    #[test]
    fn two_words_single_quoted() {
        ExpectedParseResult::new(&"tag:\'foo bar\'", 0, 1, 1, &"", 13, 1, 14)
            .compare(&tagged, &r#"tag:'foo bar'"#)
    }

    #[test]
    fn two_words_double_quoted() {
        ExpectedParseResult::new(&"tag:\"foo bar\"", 0, 1, 1, &"", 13, 1, 14)
            .compare(&tagged, &r#"tag:"foo bar""#)
    }

    #[test]
    fn tag_entirely_single_quoted() {
        assert!(tagged(Span::new(r#"'foo:bar'"#)).is_err())
    }

    #[test]
    fn tag_entirely_double_quoted() {
        assert!(tagged(Span::new(r#""foo:bar""#)).is_err())
    }
}

// Xapian tags in human format, e.g. "author:" or "title:"
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
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

impl fmt::Display for XapianTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", self.to_xapian())
    }
}

pub fn xapiantag(input: Span) -> IResult<(XapianTag, Span)> {
    separated_pair(
        alt((
            value(XapianTag::Author, tag_no_case("author")),
            value(XapianTag::Date, tag_no_case("date")),
            value(XapianTag::Filename, tag_no_case("filename")),
            value(XapianTag::Fullpath, tag_no_case("fullpath")),
            value(XapianTag::Title, tag_no_case("title")),
            value(XapianTag::Subtitle, tag_no_case("subtitle")),
            value(XapianTag::Tag, tag_no_case("tag")),
        )),
        tag(":"),
        alt((quoted, word)),
    )(input)
}

mod xapiantag_tests {
    use super::*;
    #[test]
    fn unrecognized_tag() {
        assert!(xapiantag(Span::new(r#"foo:bar"#)).is_err())
    }

    #[test]
    fn tag_no_trailing_whitespace() {
        assert!(xapiantag(Span::new(r#"author:bar"#)).is_err())
    }

    #[test]
    fn one_word_tag() {
        let (remainder, (tag, value)) =
            xapiantag(Span::new(r#"author:bar "#)).expect("Failed to parse input");
        assert_eq!("A", tag.to_xapian());
        assert_eq!(&"bar", value.fragment());
        assert_eq!(&" ", remainder.fragment());
    }

    #[test]
    fn two_word_tag() {
        let (remainder, (tag, value)) =
            xapiantag(Span::new(r#"author:bar other"#)).expect("Failed to parse input");
        assert_eq!("A", tag.to_xapian());
        assert_eq!(&"bar", value.fragment());
        assert_eq!(&" other", remainder.fragment());
    }
}

//fn expression(input: Span) -> IResult<Span> {
//    tuple((word, matchop, alt((word, quoted))))(input)
//}

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

pub fn parse_user_query(mut qstr: LocatedSpan<&str>) -> Result<Query, Report> {
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
    //let mut operator: Option<&XapianOp> = None;
    let mut operator: Option<XapianOp> = None;
    //let mut accumulator = QueryParseState::new();

    while qstr.len() > 0 {
        //println!("Processing '{}'", qstr);

        match take_up_to_operator(qstr.as_bytes()) {
            Ok((remaining, current)) => {
                let curr_query = str::from_utf8(&current)?;
                //println!("Took Query up to operator: '{}'", curr_query);
                qstr = Span::new(str::from_utf8(&remaining)?);
                if query.is_none() {
                    let q = qp
                        .parse_query(curr_query, flags)
                        .expect("QueryParser error");
                    //println!("parsed query string '{}'", curr_query);
                    query = Some(q);
                } else {
                    //println!("appended query string {}", curr_query);
                    query = Some(
                        query
                            .unwrap()
                            .add_right(operator.unwrap(), &mut qp.parse_query(curr_query, flags)?)
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
                        .parse_query(&qstr.to_string(), flags)
                        .expect("No more operators: QueryParser error");
                    //println!("parsed query string '{}'", qstr);
                    query = Some(q);
                } else {
                    //println!("No more operators: appended query string {}", qstr);
                    query = Some(
                        query
                            .unwrap()
                            .add_right(
                                operator.unwrap(),
                                &mut qp.parse_query(&qstr.to_string(), flags)?,
                            )
                            .expect("No more operators: Failed to add_right()"),
                    );
                }
            }
        };

        //println!("MATCH OP: {}", qstr);
        match matchop(&qstr) {
            Ok((remaining, op)) => {
                // Convert MatchOp into Some(XapianOp)
                operator = Some(op.into());
                qstr = remaining
            }
            Err(_) => {
                //eprintln!("Match Op error: '{}' in '{}'", e, qstr);
                break;
            }
        };
    }

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

//named!(
//    match_xapiantag2,
//    recognize!(alt!(
//        value!(XapianTag::Author, tag!("author:"))
//            | value!(XapianTag::Date, tag!("date:"))
//            | value!(XapianTag::Filename, tag!("filename:"))
//            | value!(XapianTag::Fullpath, tag!("fullpath:"))
//            | value!(XapianTag::Title, tag!("title:"))
//            | value!(XapianTag::Subtitle, tag!("subtitle:"))
//            | value!(XapianTag::Tag, tag!("tag:"))
//    ))
//);
//
//pub fn match_op(input: &str) -> IResult<&str, &XapianOp> {
//    // Note 1:
//    // From https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md
//    // Note that case insensitive comparison is not well defined for unicode,
//    // and that you might have bad surprises
//    // Note 2:
//    // Order these by longest match, according to
//    // https://docs.rs/nom/6.2.1/nom/macro.alt.html#behaviour-of-alt
//    alt((
//        value(&XapianOp::OpAndNot, tag_no_case("AND NOT")),
//        value(&XapianOp::OpAnd, tag_no_case("AND")),
//        value(&XapianOp::OpXor, tag_no_case("XOR")),
//        value(&XapianOp::OpOr, tag_no_case("OR")),
//        // OpAndMaybe,
//        // OpFilter,
//        // OpNear,
//        // OpPhrase,
//        // OpValueRange,
//        // OpScaleWeight,
//        // OpEliteSet,
//        // OpValueGe,
//        // OpValueLe,
//        // OpSynonym,
//    ))(input)
//}

//named!(
//    match_op2,
//    recognize!(alt!(
//        value!(&XapianOp::OpAndNot, tag_no_case("AND NOT"))
//            | value!(&XapianOp::OpAnd, tag_no_case("AND"))
//            | value!(&XapianOp::OpXor, tag_no_case("XOR"))
//            | value!(&XapianOp::OpOr, tag_no_case("OR"))
//    ))
//);

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
