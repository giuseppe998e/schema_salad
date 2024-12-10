use std::{fmt, marker::PhantomData};

use salad_types::SaladType;
use serde::de;

use super::{IntoDeserializeSeed, SeedData};

pub struct ListDeserializeSeed<'sd, T> {
    pub(super) data: &'sd SeedData,
    pub(super) _phant: PhantomData<T>,
}

impl<'de, 'sd, T> de::DeserializeSeed<'de> for ListDeserializeSeed<'sd, T>
where
    T: SaladType + IntoDeserializeSeed<'de, 'sd>,
{
    type Value = Box<[T]>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ListVisitor<'sd, T> {
            data: &'sd SeedData,
            _phant: PhantomData<T>,
        }

        impl<'de, 'sd, T> de::Visitor<'de> for ListVisitor<'sd, T>
        where
            T: SaladType + IntoDeserializeSeed<'de, 'sd>,
        {
            type Value = Box<[T]>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("one or a list of values")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as i32)
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as i32)
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as i32)
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i32(v as i32)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let deserializer = de::IntoDeserializer::into_deserializer(v);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let deserializer = de::value::MapAccessDeserializer::new(map);
                de::DeserializeSeed::deserialize(T::deserialize_seed(self.data), deserializer)
                    .map(|t| Box::from([t]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let capacity = seq.size_hint().unwrap_or(0);
                let mut entries = Vec::with_capacity(capacity);

                while let Some(entry) = seq.next_element_seed(T::deserialize_seed(self.data))? {
                    entries.push(entry);
                }

                Ok(entries.into_boxed_slice())
            }
        }

        deserializer.deserialize_any(ListVisitor {
            data: self.data,
            _phant: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use salad_types::SaladAny;
    use serde::__private::de::{Content, ContentDeserializer};

    use super::*;

    #[test]
    fn single_object_entry() {
        let input = r#"
            type: object
            key: value
        "#;

        let deserializer: ContentDeserializer<'_, serde_yml::Error> = {
            let content: Content<'static> = serde_yml::from_str::<Content>(input).unwrap();
            ContentDeserializer::new(content)
        };

        assert!(de::DeserializeSeed::deserialize(
            <Box<[SaladAny]>>::deserialize_seed(&SeedData),
            deserializer
        )
        .is_ok())
    }

    #[test]
    fn single_primitive_entry() {
        let input = r#"Hello, World!"#;

        let deserializer: ContentDeserializer<'_, serde_yml::Error> = {
            let content: Content<'static> = serde_yml::from_str::<Content>(input).unwrap();
            ContentDeserializer::new(content)
        };

        assert!(de::DeserializeSeed::deserialize(
            <Box<[SaladAny]>>::deserialize_seed(&SeedData),
            deserializer
        )
        .is_ok())
    }

    #[test]
    fn multiple_entries() {
        let input = r#"
            - 1
            - 2.0
            - true
            - Hello, World!
            - type: object
              key: value
        "#;

        let deserializer: ContentDeserializer<'_, serde_yml::Error> = {
            let content: Content<'static> = serde_yml::from_str::<Content>(input).unwrap();
            ContentDeserializer::new(content)
        };

        assert!(de::DeserializeSeed::deserialize(
            <Box<[SaladAny]>>::deserialize_seed(&SeedData),
            deserializer
        )
        .is_ok())
    }
}
