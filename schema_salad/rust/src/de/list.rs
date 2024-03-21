use std::{fmt, marker::PhantomData};

use serde::{__private::de as de_private, de};

use crate::{
    core::List,
    de::{IntoDeserializeSeed, SeedData},
};

// Default logic for deserialization of `crate::core::List<T>`
impl<'de, 'sd, T> IntoDeserializeSeed<'de, 'sd> for List<T>
where
    T: IntoDeserializeSeed<'de, 'sd>,
{
    type Value = OneOrMoreDeserializeSeed<'sd, T>;

    #[inline]
    fn into_dseed(data: &'sd SeedData) -> Self::Value {
        OneOrMoreDeserializeSeed {
            data,
            _phant: PhantomData,
        }
    }
}

/// Allows to deserialize a list from one object or
/// a list of objects.
pub(crate) struct OneOrMoreDeserializeSeed<'sd, T> {
    data: &'sd SeedData,
    _phant: PhantomData<T>,
}

impl<'de, 'sd, T> de::DeserializeSeed<'de> for OneOrMoreDeserializeSeed<'sd, T>
where
    T: IntoDeserializeSeed<'de, 'sd>,
{
    type Value = List<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct OneOrMoreVisitor<'sd, T> {
            data: &'sd SeedData,
            _phant: PhantomData<T>,
        }

        impl<'de, 'sd, T> de::Visitor<'de> for OneOrMoreVisitor<'sd, T>
        where
            T: IntoDeserializeSeed<'de, 'sd>,
        {
            type Value = List<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("one or a sequence of objects")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let deserializer = de::value::MapAccessDeserializer::new(map);
                let dseed = T::into_dseed(self.data);
                let entry = de::DeserializeSeed::deserialize(dseed, deserializer)?;
                Ok(Box::new([entry]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut entries = Vec::with_capacity(seq.size_hint().unwrap_or(0));

                while let Some(entry) = seq.next_element_seed(T::into_dseed(self.data))? {
                    entries.push(entry);
                }

                Ok(entries.into_boxed_slice())
            }
        }

        deserializer.deserialize_any(OneOrMoreVisitor {
            data: self.data,
            _phant: PhantomData,
        })
    }
}

/// Allows to deserialize a map or a sequence as
/// a list of objects, in the same way.
///
/// ```yaml
///     entries:
///         - key1: value1
///           key2: value2
///           key3: value3
/// ```
/// ...equals...
/// ```yaml
///     entries:
///         value1:
///             key2: value2
///             key3: value3
/// ```
/// ...given `value1` an explicit key (eg. `key1`)
pub(crate) struct MapOrSeqDeserializeSeed<'sd, T> {
    key: &'static str,
    predicate: Option<&'static str>,
    data: &'sd SeedData,
    _phant: PhantomData<T>,
}

impl<'sd, T> MapOrSeqDeserializeSeed<'sd, T> {
    pub fn new(key: &'static str, predicate: Option<&'static str>, data: &'sd SeedData) -> Self {
        Self {
            key,
            predicate,
            data,
            _phant: PhantomData,
        }
    }
}

impl<'de, 'sd, T> de::DeserializeSeed<'de> for MapOrSeqDeserializeSeed<'sd, T>
where
    T: IntoDeserializeSeed<'de, 'sd>,
{
    type Value = List<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct MapOrSeqVisitor<'k, 's, T> {
            key: &'k str,
            predicate: Option<&'k str>,
            data: &'s SeedData,
            _phant: PhantomData<T>,
        }

        impl<'de, 'k, 's, T> de::Visitor<'de> for MapOrSeqVisitor<'k, 's, T>
        where
            T: IntoDeserializeSeed<'de, 's>,
        {
            type Value = List<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a sequence or map of objects")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));

                while let Some((key_value, value)) =
                    map.next_entry::<de_private::Content<'de>, de_private::Content<'de>>()?
                {
                    let map = match (value, self.predicate) {
                        (de_private::Content::Map(mut map), _) => {
                            let key = de_private::Content::String(self.key.to_owned());
                            map.reserve_exact(1);
                            map.push((key, key_value));
                            map
                        }
                        (value, Some(pred)) => {
                            let key = de_private::Content::String(self.key.to_owned());
                            let predicate = de_private::Content::String(pred.to_owned());
                            vec![(key, key_value), (predicate, value)]
                        }
                        (_, None) => {
                            return Err(de::Error::custom(
                                "data type cannot be deserialized as a list of objects",
                            ))
                        }
                    };

                    let entry = {
                        let content = de_private::Content::Map(map);
                        let deserializer = de_private::ContentDeserializer::new(content);
                        let entry_dseed = T::into_dseed(self.data);
                        de::DeserializeSeed::deserialize(entry_dseed, deserializer)?
                    };

                    entries.push(entry);
                }

                Ok(entries.into_boxed_slice())
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut entries = Vec::with_capacity(seq.size_hint().unwrap_or(0));

                while let Some(entry) = seq.next_element_seed(T::into_dseed(self.data))? {
                    entries.push(entry);
                }

                Ok(entries.into_boxed_slice())
            }
        }

        deserializer.deserialize_any(MapOrSeqVisitor {
            key: self.key,
            predicate: self.predicate,
            data: self.data,
            _phant: PhantomData,
        })
    }
}
