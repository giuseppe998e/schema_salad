use std::{fmt, marker::PhantomData};

use compact_str::CompactString;
use salad_types::SaladType;
use serde::de;

use super::{IntoDeserializeSeed, SeedData};

/// TODO ...
pub struct MapDeserializeSeed<'sd, T> {
    key: &'static str,
    pred: Option<&'static str>,
    data: &'sd SeedData,
    _phant: PhantomData<T>,
}

impl<'de, 'sd, T> MapDeserializeSeed<'sd, T>
where
    T: SaladType + IntoDeserializeSeed<'de, 'sd>,
{
    /// TODO ...
    pub fn new(key: &'static str, pred: Option<&'static str>, data: &'sd SeedData) -> Self {
        Self {
            key,
            pred,
            data,
            _phant: PhantomData,
        }
    }
}

impl<'de, 'sd, T> de::DeserializeSeed<'de> for MapDeserializeSeed<'sd, T>
where
    T: SaladType + IntoDeserializeSeed<'de, 'sd>,
{
    type Value = Box<[T]>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct MapVisitor<'sd, T> {
            data: &'sd SeedData,
            _phant: PhantomData<T>,
        }

        impl<'de, 'sd, T> de::Visitor<'de> for MapVisitor<'sd, T>
        where
            T: SaladType + IntoDeserializeSeed<'de, 'sd>,
        {
            type Value = Box<[T]>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("one or a list of objects")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                use serde::__private::de::Content;

                let capacity = map.size_hint().unwrap_or(0);
                let mut entries = Vec::with_capacity(capacity);

                while let Some((key, value)) = map.next_entry::<CompactString, Content<'de>>()? {



                }

                // TODO

                Ok(entries.into_boxed_slice())
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

        deserializer.deserialize_any(MapVisitor {
            data: self.data,
            _phant: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {}
