use chrono::{DateTime, FixedOffset};
use color_eyre::Report;
use eyre::{eyre, Result};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::io::{Error, ErrorKind};
use std::{ffi::OsString, fmt, fs, io, marker::PhantomData};
use yaml_rust::YamlEmitter;

/// Representation for a given Markdown + FrontMatter file; Example:
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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct TikaDocument {
    /// Inherent metadata about the document
    #[serde(default)]
    pub filename: String,
    #[serde(skip_deserializing)]
    pub full_path: OsString,

    /// FrontMatter-derived metadata about the document
    #[serde(default)]
    pub author: String,
    pub date: String,

    /// RFC 3339 based timestamp
    #[serde(deserialize_with = "string_or_list_string")]
    pub tags: Vec<String>,
    pub title: String,

    /// The Markdown-formatted body of the document
    #[serde(skip_deserializing)]
    pub body: String,
}

impl TikaDocument {
    pub(crate) fn parse_date(&self) -> Result<DateTime<FixedOffset>, Report> {
        if let Ok(rfc3339) = DateTime::parse_from_rfc3339(&self.date) {
            return Ok(rfc3339);
        } else if let Ok(s) = DateTime::parse_from_str(&self.date, &String::from("%Y-%m-%dT%T%z")) {
            return Ok(s);
        }
        eprintln!("❌ Failed to convert path to str '{}'", &self.filename);
        Err(eyre!(
            "❌ Failed to convert path to str '{}'",
            &self.filename
        ))
    }
}

/// Support Deserializing a string into a list of string of length 1
fn string_or_list_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<String>>);

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or list of strings")
        }

        // Value is a single string: return a Vec containing that single string
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_owned()])
        }

        fn visit_seq<S>(self, visitor: S) -> Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(visitor))
        }
    }

    deserializer.deserialize_any(StringOrVec(PhantomData))
}

pub(crate) fn parse_file(path: &std::path::PathBuf) -> Result<TikaDocument, io::Error> {
    let s = fs::read_to_string(path.to_str().unwrap())?;

    let (yaml, content) = frontmatter::parse_and_find_content(&s).unwrap();
    match yaml {
        Some(yaml) => {
            let mut out_str = String::new();
            {
                let mut emitter = YamlEmitter::new(&mut out_str);
                emitter.dump(&yaml).unwrap(); // dump the YAML object to a String
            }

            let mut doc: TikaDocument = serde_yaml::from_str(&out_str).unwrap();
            if doc.filename == *"" {
                doc.filename = String::from(path.file_name().unwrap().to_str().unwrap());
            }

            doc.body = content.to_string();

            Ok(doc)
        }
        None => Err(Error::new(
            ErrorKind::Other,
            format!("Failed to process file {}", path.display()),
        )),
    }
}

//impl From<TantivyDoc> for TikaDocument {
//    fn from(item: TantivyDoc) -> Self {
//        TikaDocument {
//            filename: item
//                .retrieved_doc
//                .get_first(item.filename)
//                .unwrap()
//                .text()
//                .unwrap_or("")
//                .into(),
//            full_path: item
//                .retrieved_doc
//                .get_first(item.full_path)
//                .unwrap()
//                .text()
//                .unwrap_or("")
//                .into(),
//            author: item
//                .retrieved_doc
//                .get_first(item.author)
//                .unwrap()
//                .text()
//                .unwrap_or("")
//                .into(),
//            title: item
//                .retrieved_doc
//                .get_first(item.title)
//                .unwrap()
//                .text()
//                .unwrap_or("")
//                .into(),
//            body: String::from(""),
//            date: item
//                .retrieved_doc
//                .get_first(item.date)
//                .unwrap()
//                .text()
//                .unwrap_or("")
//                .into(),
//            tags: vec![String::from("foo")],
//        }
//    }
//}
//
//struct TantivyDoc {
//    retrieved_doc: Document,
//    author: Field,
//    date: Field,
//    filename: Field,
//    full_path: Field,
//    //tags: Field,
//    title: Field,
//}