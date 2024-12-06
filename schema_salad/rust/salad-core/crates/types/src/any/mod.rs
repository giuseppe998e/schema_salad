mod deser;
mod object;

use std::{fmt, ops::RangeInclusive};

use serde::{de, ser, Deserialize};

pub use self::object::SaladObject;
use crate::{
    primitive::{SaladBool, SaladDouble, SaladFloat, SaladInt, SaladLong, SaladString},
    SaladType, SaladTypeDowncastError,
};

const INT_RANGE: RangeInclusive<i64> = SaladInt::MIN as SaladLong..=SaladInt::MAX as SaladLong;
const FLOAT_RANGE: RangeInclusive<f64> =
    SaladFloat::MIN as SaladDouble..=SaladFloat::MAX as SaladDouble;

/// The `SaladAny` type validates for any non-null value.
#[derive(Debug, Clone)]
pub enum SaladAny {
    /// A binary value.
    Bool(SaladBool),
    /// 32-bit signed integer.
    Int(SaladInt),
    /// 64-bit signed integer.
    Long(SaladLong),
    /// Single precision (32-bit) IEEE 754 floating-point number.
    Float(SaladFloat),
    /// Double precision (64-bit) IEEE 754 floating-point number.
    Double(SaladDouble),
    /// Unicode character sequence.
    String(SaladString),
    /// Unknown object.
    Object(SaladObject),
    /// List of any values.
    List(Box<[SaladAny]>),
}

impl SaladAny {
    /// Attempts to downcast to type `T` from a borrowed `SaladAny`.
    /// N.B. When downcasting to a primitive (or object) type, consider using
    /// a `match` expression or the `TryFrom::try_from` method.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    pub fn downcast<'de, T>(&'de self) -> Result<T, SaladTypeDowncastError>
    where
        T: SaladType + de::Deserialize<'de>,
    {
        let deserializer = self::deser::SaladAnyDeserializer(self);
        T::deserialize(deserializer)
    }

    /// Attempts to downcast from a consumed `SaladAny` to type `T`.
    /// N.B. When downcasting to a primitive (or object) type, consider using
    /// a `match` expression or the `TryFrom::try_from` method.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    #[inline]
    pub fn downcast_into<T>(self) -> Result<T, SaladTypeDowncastError>
    where
        for<'de> T: SaladType + de::Deserialize<'de>,
    {
        Self::downcast(&self)
    }
}

impl SaladType for SaladAny {}

crate::util::impl_from_traits! {
    (SaladAny, SaladTypeDowncastError)

    Bool => SaladBool,
    Int => SaladInt,
    Long => SaladLong,
    Float => SaladFloat,
    Double => SaladDouble,
    String => SaladString,
    Object => SaladObject,
}

impl ser::Serialize for SaladAny {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Int(i) => serializer.serialize_i32(*i),
            Self::Long(l) => serializer.serialize_i64(*l),
            Self::Float(f) => serializer.serialize_f32(*f),
            Self::Double(d) => serializer.serialize_f64(*d),
            Self::String(s) => s.serialize(serializer),
            Self::Object(o) => o.serialize(serializer),
            Self::List(l) => l.serialize(serializer),
        }
    }
}

impl<'de> de::Deserialize<'de> for SaladAny {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SaladAnyVisitor;

        impl<'de> de::Visitor<'de> for SaladAnyVisitor {
            type Value = SaladAny;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a salad primitive, a key-value object, or a list of them")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(SaladAny::Bool(v))
            }

            fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
                Ok(SaladAny::Int(v as i32))
            }

            fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
                Ok(SaladAny::Int(v as i32))
            }

            fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
                Ok(SaladAny::Int(v))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                match v {
                    l if INT_RANGE.contains(&l) => Ok(SaladAny::Int(v as i32)),
                    _ => Ok(SaladAny::Long(v)),
                }
            }

            fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
                Ok(SaladAny::Int(v as i32))
            }

            fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
                Ok(SaladAny::Int(v as i32))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                match v {
                    u if u <= i32::MAX as u64 => Ok(SaladAny::Int(v as i32)),
                    u if u <= i64::MAX as u64 => Ok(SaladAny::Long(v as i64)),
                    _ => Err(de::Error::invalid_value(de::Unexpected::Unsigned(v), &self)),
                }
            }

            fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
                Ok(SaladAny::Float(v))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                match v {
                    d if FLOAT_RANGE.contains(&d) => Ok(SaladAny::Float(v as f32)),
                    _ => Ok(SaladAny::Double(v)),
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(SaladAny::String(v.into()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(SaladAny::String(v.into()))
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                match std::str::from_utf8(v) {
                    Ok(s) => Ok(SaladAny::String(s.into())),
                    Err(_) => Err(de::Error::invalid_value(de::Unexpected::Bytes(v), &self)),
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let deserializer = de::value::MapAccessDeserializer::new(map);
                SaladObject::deserialize(deserializer).map(SaladAny::Object)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let deserializer = de::value::SeqAccessDeserializer::new(seq);
                Box::<[SaladAny]>::deserialize(deserializer).map(SaladAny::List)
            }
        }

        deserializer.deserialize_any(SaladAnyVisitor)
    }
}
