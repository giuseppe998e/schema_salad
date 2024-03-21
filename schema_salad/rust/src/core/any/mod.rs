mod object;

use std::fmt;

use serde::{de, ser};

pub use self::object::Object;
use crate::{
    core::{Bool, Double, Float, Int, List, Long, SaladType, StrValue},
    de::{IntoDeserializeSeed, SeedData},
};

#[derive(Debug, Clone)]
pub enum Any {
    Bool(Bool),
    Double(Double),
    Float(Float),
    Int(Int),
    Long(Long),
    String(StrValue),
    Object(Object),
    List(List<Self>),
}

impl SaladType for Any {}

impl From<Bool> for Any {
    fn from(value: Bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Double> for Any {
    fn from(value: Double) -> Self {
        Self::Double(value)
    }
}

impl From<Float> for Any {
    fn from(value: Float) -> Self {
        Self::Float(value)
    }
}

impl From<Int> for Any {
    fn from(value: Int) -> Self {
        Self::Int(value)
    }
}

impl From<Long> for Any {
    fn from(value: Long) -> Self {
        Self::Long(value)
    }
}

impl From<StrValue> for Any {
    fn from(value: StrValue) -> Self {
        Self::String(value)
    }
}

impl From<Object> for Any {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}

impl ser::Serialize for Any {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Self::Bool(v) => v.serialize(serializer),
            Self::Double(v) => v.serialize(serializer),
            Self::Float(v) => v.serialize(serializer),
            Self::Int(v) => v.serialize(serializer),
            Self::Long(v) => v.serialize(serializer),
            Self::String(v) => v.serialize(serializer),

            Self::Object(o) => o.serialize(serializer),
            Self::List(l) => l.serialize(serializer),
        }
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for Any {
    type Value = AnySeed<'sd>;

    #[inline]
    fn into_dseed(data: &'sd SeedData) -> Self::Value {
        AnySeed(data)
    }
}

pub(crate) struct AnySeed<'sd>(&'sd SeedData);

impl<'de, 'sd> de::DeserializeSeed<'de> for AnySeed<'sd> {
    type Value = Any;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        impl<'de, 'sd> de::Visitor<'de> for AnySeed<'sd> {
            type Value = Any;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any possible deserializable value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Any::Bool(v))
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as _)
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as _)
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as _)
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as _)
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Any::Int(v))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                const I32_MAX: u32 = i32::MAX as _;

                match v {
                    ..=I32_MAX => self.visit_i32(v as _),
                    _ => self.visit_i64(v as _),
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                const I32_MIN: i64 = i32::MIN as _;
                const I32_MAX: i64 = i32::MAX as _;

                match v {
                    I32_MIN..=I32_MAX => self.visit_i32(v as _),
                    _ => Ok(Any::Long(v)),
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                const I32_MAX: u64 = i32::MAX as _;
                const I32_MAX_OF: u64 = I32_MAX + 1;
                const I64_MAX: u64 = i64::MAX as _;

                match v {
                    ..=I32_MAX => self.visit_i32(v as _),
                    I32_MAX_OF..=I64_MAX => self.visit_i64(v as _),
                    _ => Err(de::Error::invalid_type(de::Unexpected::Unsigned(v), &self)),
                }
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Any::Float(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                const F32_MIN: f64 = f32::MIN as _;
                const F32_MAX: f64 = f32::MAX as _;

                if (F32_MIN..=F32_MAX).contains(&v) {
                    self.visit_f32(v as _)
                } else {
                    Ok(Any::Double(v))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Any::String(v.into()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Any::String(v.into()))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match std::str::from_utf8(v) {
                    Ok(s) => Ok(Any::String(s.into())),
                    Err(_) => Err(de::Error::invalid_value(de::Unexpected::Bytes(v), &self)),
                }
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match String::from_utf8(v) {
                    Ok(s) => Ok(Any::String(s.into())),
                    Err(e) => Err(de::Error::invalid_value(
                        de::Unexpected::Bytes(&e.into_bytes()),
                        &self,
                    )),
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let deserializer = de::value::MapAccessDeserializer::new(map);
                let seed = <Object as IntoDeserializeSeed>::into_dseed(self.0);
                de::DeserializeSeed::deserialize(seed, deserializer).map(Any::Object)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let deserializer = de::value::SeqAccessDeserializer::new(seq);
                let seed = <List<Any> as IntoDeserializeSeed>::into_dseed(self.0);
                de::DeserializeSeed::deserialize(seed, deserializer).map(Any::List)
            }
        }

        deserializer.deserialize_any(self)
    }
}
