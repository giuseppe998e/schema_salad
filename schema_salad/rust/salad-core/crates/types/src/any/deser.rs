use std::{collections::hash_map, slice};

use compact_str::CompactString;
use serde::de;

use super::{SaladAny, SaladObject, SaladTypeDowncastError};

/// TODO ...
pub(super) struct SaladAnyDeserializer<'de>(pub &'de SaladAny);

impl<'de> de::Deserializer<'de> for SaladAnyDeserializer<'de> {
    type Error = SaladTypeDowncastError;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            SaladAny::Bool(b) => visitor.visit_bool(*b),
            SaladAny::Int(i) => visitor.visit_i32(*i),
            SaladAny::Long(l) => {
                if super::INT_RANGE.contains(l) {
                    visitor.visit_i32(*l as i32)
                } else {
                    visitor.visit_i64(*l)
                }
            }
            SaladAny::Float(f) => visitor.visit_f32(*f),
            SaladAny::Double(d) => {
                if super::FLOAT_RANGE.contains(d) {
                    visitor.visit_f32(*d as f32)
                } else {
                    visitor.visit_f64(*d)
                }
            }
            SaladAny::String(s) => visitor.visit_str(s),
            SaladAny::Object(o) => visitor.visit_map(SaladObjectMapAccess::new(o)),
            SaladAny::List(l) => visitor.visit_seq(SaladAnyListSeqAccess::new(l)),
        }
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"boolean";

        match self.0 {
            SaladAny::Bool(b) => visitor.visit_bool(*b),
            SaladAny::Int(1) | SaladAny::Long(1) => visitor.visit_bool(true),
            SaladAny::Int(0) | SaladAny::Long(0) => visitor.visit_bool(false),

            // Errors
            SaladAny::Int(i) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(*i as i64),
                ERR_MSG,
            )),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::Float(f) => Err(de::Error::invalid_type(
                de::Unexpected::Float(*f as f64),
                ERR_MSG,
            )),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"signed integer";

        match self.0 {
            SaladAny::Int(i) => visitor.visit_i32(*i),
            SaladAny::Long(l) if super::INT_RANGE.contains(l) => visitor.visit_i32(*l as i32),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::Float(f) => Err(de::Error::invalid_type(
                de::Unexpected::Float(*f as f64),
                ERR_MSG,
            )),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"signed long integer";

        match self.0 {
            SaladAny::Long(l) => visitor.visit_i64(*l),
            SaladAny::Int(i) => visitor.visit_i64(*i as i64),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Float(f) => Err(de::Error::invalid_type(
                de::Unexpected::Float(*f as f64),
                ERR_MSG,
            )),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"float";

        match self.0 {
            SaladAny::Float(f) => visitor.visit_f32(*f),
            SaladAny::Double(d) if super::FLOAT_RANGE.contains(d) => visitor.visit_f32(*d as f32),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Int(i) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(*i as i64),
                ERR_MSG,
            )),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"double";

        match self.0 {
            SaladAny::Double(d) => visitor.visit_f64(*d),
            SaladAny::Float(f) => visitor.visit_f64(*f as f64),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Int(i) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(*i as i64),
                ERR_MSG,
            )),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"UTF-8 string";

        match self.0 {
            SaladAny::String(s) => visitor.visit_str(s),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Int(i) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(*i as i64),
                ERR_MSG,
            )),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::Float(f) => Err(de::Error::invalid_type(
                de::Unexpected::Float(*f as f64),
                ERR_MSG,
            )),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"key-value map object";

        match self.0 {
            SaladAny::Object(o) => visitor.visit_map(SaladObjectMapAccess::new(o)),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Int(i) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(*i as i64),
                ERR_MSG,
            )),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::Float(f) => Err(de::Error::invalid_type(
                de::Unexpected::Float(*f as f64),
                ERR_MSG,
            )),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::List(_) => Err(de::Error::invalid_type(de::Unexpected::Seq, ERR_MSG)),
        }
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        static ERR_MSG: &&str = &"list of primitives/objects";

        match self.0 {
            SaladAny::List(l) => visitor.visit_seq(SaladAnyListSeqAccess::new(l)),

            // Errors
            SaladAny::Bool(b) => Err(de::Error::invalid_type(de::Unexpected::Bool(*b), ERR_MSG)),
            SaladAny::Int(i) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(*i as i64),
                ERR_MSG,
            )),
            SaladAny::Long(l) => Err(de::Error::invalid_type(de::Unexpected::Signed(*l), ERR_MSG)),
            SaladAny::Float(f) => Err(de::Error::invalid_type(
                de::Unexpected::Float(*f as f64),
                ERR_MSG,
            )),
            SaladAny::Double(d) => Err(de::Error::invalid_type(de::Unexpected::Float(*d), ERR_MSG)),
            SaladAny::String(s) => Err(de::Error::invalid_type(de::Unexpected::Str(s), ERR_MSG)),
            SaladAny::Object(_) => Err(de::Error::invalid_type(de::Unexpected::Map, ERR_MSG)),
        }
    }

    // Unimplemented/Unnecessary

    fn deserialize_i8<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i16<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u8<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u16<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u32<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u64<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_unit<V: de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_tuple<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_identifier<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }
}

/// TODO ...
pub(super) struct SaladObjectMapAccess<'de> {
    iter: hash_map::Iter<'de, CompactString, SaladAny>,
    value: Option<&'de SaladAny>,
}

impl<'de> SaladObjectMapAccess<'de> {
    pub fn new(obj: &'de SaladObject) -> Self {
        Self {
            iter: obj.map.iter(),
            value: None,
        }
    }
}

impl<'de> de::Deserializer<'de> for SaladObjectMapAccess<'de> {
    type Error = SaladTypeDowncastError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    // Unimplemented/Unnecessary

    fn deserialize_bool<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i8<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i16<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u8<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u16<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_char<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_str<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_string<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_bytes<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_option<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_unit<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_seq<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_tuple<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_identifier<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }
}

impl<'de> de::MapAccess<'de> for SaladObjectMapAccess<'de> {
    type Error = SaladTypeDowncastError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((k, v)) => {
                self.value = Some(v);
                seed.deserialize(CompactStringDeserializer(k)).map(Some)
            }
            None => {
                self.value = None;
                Ok(None)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let value = self.value.ok_or(SaladTypeDowncastError::new())?;
        seed.deserialize(SaladAnyDeserializer(value))
    }
}

/// TODO ...
struct CompactStringDeserializer<'de>(&'de CompactString);

impl<'de> de::Deserializer<'de> for CompactStringDeserializer<'de> {
    type Error = SaladTypeDowncastError;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_str(self.0.as_str())
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_str(self.0.as_str())
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.0.to_string())
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_bytes(self.0.as_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let byte_buf = Vec::from(self.0.as_bytes());
        visitor.visit_byte_buf(byte_buf)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.0.as_str())
    }

    // Unimplemented/Unnecessary
    // (If `V::Value` was generated by the `schema_salad` tool,
    //  it will never requires any of the following methods)

    fn deserialize_bool<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i8<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i16<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_i64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u8<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u16<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_u64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_char<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_option<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_unit<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_seq<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_tuple<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_map<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }

    fn deserialize_ignored_any<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(SaladTypeDowncastError::new())
    }
}

/// TODO ...
struct SaladAnyListSeqAccess<'de> {
    iter: slice::Iter<'de, SaladAny>,
}

impl<'de> SaladAnyListSeqAccess<'de> {
    pub fn new(list: &'de [SaladAny]) -> Self {
        Self { iter: list.iter() }
    }
}

impl<'de> de::SeqAccess<'de> for SaladAnyListSeqAccess<'de> {
    type Error = SaladTypeDowncastError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(|v| seed.deserialize(SaladAnyDeserializer(v)))
            .transpose()
    }
}
