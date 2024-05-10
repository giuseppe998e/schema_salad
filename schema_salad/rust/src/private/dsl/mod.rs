#[cfg(all(
    feature = "dsl",
    not(any(feature = "dsl_json", feature = "dsl_yaml"))
))]
compile_error!(
    "feature \"dsl\" requires at least one feature between \"dsl_json\" and \"dsl_yaml\""
);

mod map_access;

use std::{fmt, fs};

use compact_str::CompactString;
use serde::de::{self, MapAccess};

use self::map_access::PeekableMapAccess;

pub(crate) struct Preprocessor<T> {
    delegate: T,
}

impl<'de, D: de::Deserializer<'de>> Preprocessor<D> {
    pub fn new(delegate: D) -> Self {
        Self { delegate }
    }
}

impl<'de, V: de::Visitor<'de>> Preprocessor<V> {
    #[cfg(feature = "dsl_json")]
    fn deserialize_json_data<E: de::Error>(self, data: String) -> Result<V::Value, E> {
        let data_cursor = std::io::Cursor::new(data);
        let deserializer = &mut serde_json::Deserializer::from_reader(data_cursor);
        de::Deserializer::deserialize_any(deserializer, self)
            .map_err(|e| E::custom(format_args!("preprocessor error: {e}")))
    }

    #[cfg(feature = "dsl_yaml")]
    fn deserialize_yaml_data<E: de::Error>(self, data: String) -> Result<V::Value, E> {
        let data_cursor = std::io::Cursor::new(data);
        let deserializer = serde_yaml::Deserializer::from_reader(data_cursor);
        de::Deserializer::deserialize_any(deserializer, self)
            .map_err(|e| E::custom(format_args!("preprocessor error: {e}")))
    }
}

impl<'de, V: de::Visitor<'de>> de::Visitor<'de> for Preprocessor<V> {
    type Value = V::Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.delegate.expecting(f)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        self.delegate.visit_bool(v)
    }

    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        self.delegate.visit_i8(v)
    }

    fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
        self.delegate.visit_i16(v)
    }

    fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
        self.delegate.visit_i32(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        self.delegate.visit_i64(v)
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Self::Value, E> {
        self.delegate.visit_i128(v)
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        self.delegate.visit_u8(v)
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        self.delegate.visit_u16(v)
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        self.delegate.visit_u32(v)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        self.delegate.visit_u64(v)
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
        self.delegate.visit_u128(v)
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        self.delegate.visit_f32(v)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        self.delegate.visit_f64(v)
    }

    fn visit_char<E: de::Error>(self, v: char) -> Result<Self::Value, E> {
        self.delegate.visit_char(v)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        self.delegate.visit_str(v)
    }

    fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
        self.delegate.visit_borrowed_str(v)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.delegate.visit_string(v)
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        self.delegate.visit_bytes(v)
    }

    fn visit_borrowed_bytes<E: de::Error>(self, v: &'de [u8]) -> Result<Self::Value, E> {
        self.delegate.visit_borrowed_bytes(v)
    }

    fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        self.delegate.visit_byte_buf(v)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.delegate.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.delegate.visit_some(Preprocessor::new(deserializer))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.delegate
            .visit_newtype_struct(Preprocessor::new(deserializer))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.delegate.visit_unit()
    }

    fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        fn get_uri_extension(uri: &str) -> Option<&str> {
            uri.split_once(['?', '#'])
                .map(|(url, _)| url)
                .or(Some(uri))
                .and_then(|uri| uri.rsplit_once('.'))
                .map(|(_, ext)| ext)
        }

        #[cfg_attr(not(feature = "dsl_http"), inline)]
        fn read_uri_content<E: de::Error>(uri: &str) -> Result<String, E> {
            #[cfg(feature = "dsl_http")]
            if uri.starts_with("https://") || uri.starts_with("http://") {
                return match ureq::get(uri).call() {
                    Ok(r) => match r.status() {
                        200 => r.into_string().map_err(|e| E::custom(format_args!("preprocessor error: {e}"))),
                        code => Err(E::custom(format_args!(
                            "preprocessor error: request to `{uri}` failed with status code: {code} - {}",
                            r.status_text(),
                        ))),
                    },
                    Err(e) => Err(E::custom(format_args!("preprocessor error: {e}"))),
                };
            }

            fs::read_to_string(uri).map_err(|e| E::custom(format_args!("preprocessor error: {e}")))
        }

        // "visit_map" method
        let mut peekable_map = PeekableMapAccess::new(map);
        let key_value = peekable_map.peek_key::<CompactString>()?;

        match key_value.as_deref() {
            Some("$import") => {
                let uri = peekable_map.next_value::<String>()?;

                match get_uri_extension(&uri) {
                    #[cfg(all(feature = "dsl_json", feature = "dsl_http"))]
                    Some("json" | "JSON") | None => {
                        let content = read_uri_content(&uri)?;
                        self.deserialize_json_data(content)
                    }
                    #[cfg(all(feature = "dsl_json", not(feature = "dsl_http")))]
                    Some("json" | "JSON") => {
                        let content = read_uri_content(&uri)?;
                        self.deserialize_json_data(content)
                    }
                    #[cfg(feature = "dsl_yaml")]
                    Some("yaml" | "yml" | "YAML" | "YML") => {
                        let content = read_uri_content(&uri)?;
                        self.deserialize_yaml_data(content)
                    }
                    Some(ext) => Err(<A::Error as de::Error>::custom(format_args!(
                        "preprocessor error: deserializer for `{ext}` file type missing."
                    ))),
                    #[cfg(not(feature = "dsl_http"))]
                    None => Err(<A::Error as de::Error>::custom(format_args!(
                        "preprocessor error: `$import` file type missing."
                    ))),
                }
            }
            Some("$include") => {
                let uri = peekable_map.next_value::<String>()?;
                let content = read_uri_content(&uri)?;
                self.delegate.visit_string(content)
            }
            _ => self.delegate.visit_map(Preprocessor::__new(peekable_map)),
        }
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        self.delegate.visit_seq(Preprocessor::__new(seq))
    }

    fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        self.delegate.visit_enum(Preprocessor::__new(data))
    }
}

impl<'de, A> de::MapAccess<'de> for Preprocessor<A>
where
    A: de::MapAccess<'de>,
{
    type Error = A::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.delegate.next_key_seed(Preprocessor::__new(seed))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.delegate.next_value_seed(Preprocessor::__new(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.delegate.size_hint()
    }
}

impl<'de, A: de::SeqAccess<'de>> de::SeqAccess<'de> for Preprocessor<A> {
    type Error = A::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.delegate.next_element_seed(Preprocessor::__new(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.delegate.size_hint()
    }
}

impl<'de, A: de::EnumAccess<'de>> de::EnumAccess<'de> for Preprocessor<A> {
    type Error = A::Error;
    type Variant = Preprocessor<A::Variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.delegate
            .variant_seed(seed)
            .map(|(v1, v2)| (v1, Preprocessor::__new(v2)))
    }
}

impl<'de, A: de::VariantAccess<'de>> de::VariantAccess<'de> for Preprocessor<A> {
    type Error = A::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.delegate.unit_variant()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.delegate
            .newtype_variant_seed(Preprocessor::__new(seed))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .tuple_variant(len, Preprocessor::__new(visitor))
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .struct_variant(fields, Preprocessor::__new(visitor))
    }
}

impl<'de, S: de::DeserializeSeed<'de>> de::DeserializeSeed<'de> for Preprocessor<S> {
    type Value = S::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.delegate.deserialize(Preprocessor::new(deserializer))
    }
}

impl<'de, D: de::Deserializer<'de>> de::Deserializer<'de> for Preprocessor<D> {
    type Error = D::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_any(Preprocessor::__new(visitor))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_bool(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_i8(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_i64(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_i128(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_u8(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_u16(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_u64(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_u128(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_f64(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_char(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_option(Preprocessor::__new(visitor))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_unit_struct(name, Preprocessor::__new(visitor))
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_newtype_struct(name, Preprocessor::__new(visitor))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_seq(Preprocessor::__new(visitor))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_tuple(len, Preprocessor::__new(visitor))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_tuple_struct(name, len, Preprocessor::__new(visitor))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_map(Preprocessor::__new(visitor))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_struct(name, fields, Preprocessor::__new(visitor))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_enum(name, variants, Preprocessor::__new(visitor))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate.deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.delegate
            .deserialize_ignored_any(Preprocessor::__new(visitor))
    }

    fn is_human_readable(&self) -> bool {
        self.delegate.is_human_readable()
    }
}

impl<T> Preprocessor<T> {
    fn __new(delegate: T) -> Self {
        Self { delegate }
    }
}
