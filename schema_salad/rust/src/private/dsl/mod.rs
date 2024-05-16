#[cfg(all(feature = "dsl", not(any(feature = "dsl_json", feature = "dsl_yaml"))))]
compile_error!(
    "feature \"dsl\" requires at least one feature between \"dsl_json\" and \"dsl_yaml\""
);

mod de_impl;
mod map_access;

use std::fs;

use compact_str::CompactString;
use serde::de::{self, MapAccess};

use self::map_access::PeekableMapAccess;
pub(crate) struct Preprocessor<T> {
    delegate: T,
}

impl<T> Preprocessor<T> {
    fn __new(delegate: T) -> Self {
        Self { delegate }
    }
}

impl<'de, V: de::Visitor<'de>> Preprocessor<V> {
    fn __visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<V::Value, A::Error> {
        let mut map = PeekableMapAccess::new(map);
        let key_value = map.peek_key::<CompactString>()?;

        match key_value.as_deref() {
            Some("$import") => {
                let uri = map.next_value::<CompactString>()?;
                let (filename, extension) = util::get_uri_filename(&uri);

                match extension {
                    #[cfg(all(feature = "dsl_json", feature = "dsl_http"))]
                    Some("json" | "JSON") | None => {
                        let content = util::read_uri_content(&uri)?;
                        self.deserialize_json(filename, content)
                    }
                    #[cfg(all(feature = "dsl_json", not(feature = "dsl_http")))]
                    Some("json" | "JSON") => {
                        let content = read_uri_content(&uri)?;
                        self.deserialize_json(filename, content)
                    }
                    #[cfg(feature = "dsl_yaml")]
                    Some("yaml" | "yml" | "YAML" | "YML") => {
                        let content = util::read_uri_content(&uri)?;
                        self.deserialize_yaml(filename, content)
                    }
                    Some(ext) => Err(<A::Error as de::Error>::custom(format_args!(
                        "preprocessor error: deserializer for `{ext}` file type missing."
                    ))),
                    #[cfg(not(feature = "dsl_http"))]
                    None => Err(<A::Error as de_impl::Error>::custom(format_args!(
                        "preprocessor error: `$import` file type missing."
                    ))),
                }
            }
            Some("$include") => {
                let uri = map.next_value::<CompactString>()?;
                let content = util::read_uri_content(&uri)?;
                self.delegate.visit_string(content)
            }
            _ => self.delegate.visit_map(Preprocessor::__new(map)),
        }
    }

    #[cfg(feature = "dsl_json")]
    fn deserialize_json<E>(self, _filename: &str, data: String) -> Result<V::Value, E>
    where
        E: de::Error,
    {
        let data_cursor = std::io::Cursor::new(data);
        let deserializer = &mut serde_json::Deserializer::from_reader(data_cursor);
        de::Deserializer::deserialize_any(deserializer, self)
            .map_err(|e| E::custom(format_args!("preprocessor error: {e}")))
    }

    #[cfg(feature = "dsl_yaml")]
    fn deserialize_yaml<E>(self, _filename: &str, data: String) -> Result<V::Value, E>
    where
        E: de::Error,
    {
        let data_cursor = std::io::Cursor::new(data);
        let deserializer = serde_yaml::Deserializer::from_reader(data_cursor);
        de::Deserializer::deserialize_any(deserializer, self)
            .map_err(|e| E::custom(format_args!("preprocessor error: {e}")))
    }
}

mod util {
    use super::*;

    pub(super) fn get_uri_filename(uri: &str) -> (&str, Option<&str>) {
        let query_frag_idx = uri.find(['?', '#']).unwrap_or(uri.len());
        let uri = &uri[..query_frag_idx];
        let path_separator_idx = uri.rfind(['/', '\\']).map(|v| v + 1).unwrap_or(0);

        let file = &uri[path_separator_idx..];
        let ext = file
            .find('.')
            .and_then(|dot_idx| (dot_idx < file.len() - 1).then_some(&file[dot_idx + 1..]));

        (file, ext) // FIXME file could be an empty string
    }

    #[cfg_attr(not(feature = "dsl_http"), inline)]
    pub(super) fn read_uri_content<E: de::Error>(uri: &str) -> Result<String, E> {
        #[cfg(feature = "dsl_http")]
        if uri.starts_with("https://") || uri.starts_with("http://") {
            return match ureq::get(uri).call() {
                Ok(r) if r.status() == 200 => r
                    .into_string()
                    .map_err(|e| E::custom(format_args!("preprocessor error: {e}"))),

                Ok(r) => Err(E::custom(format_args!(
                    "preprocessor error: request to `{uri}` failed with code {}: {}",
                    r.status(),
                    r.status_text(),
                ))),
                Err(e) => Err(E::custom(format_args!("preprocessor error: {e}"))),
            };
        }

        fs::read_to_string(uri).map_err(|e| E::custom(format_args!("preprocessor error: {e}")))
    }
}
