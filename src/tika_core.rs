use serde::{de, Deserialize, Deserializer, Serialize};
use std::{ffi::OsString, fmt, fs, io, io::Read, marker::PhantomData, path::Path};
//use std::convert::From;
//use std::io::{Error, ErrorKind};

/// Representation for a given Markdown + FrontMatter file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct TikaDocument {
    /// Inherent metadata about the document
    #[serde(default)]
    filename: String,
    #[serde(skip_deserializing)]
    pub full_path: OsString,

    /// FrontMatter-derived metadata about the document
    #[serde(default)]
    author: String,
    date: String,

    /// RFC 3339 based timestamp
    #[serde(deserialize_with = "string_or_list_string")]
    tags: Vec<String>,
    title: String,

    /// The Markdown-formatted body of the document
    #[serde(skip_deserializing)]
    body: String,
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
