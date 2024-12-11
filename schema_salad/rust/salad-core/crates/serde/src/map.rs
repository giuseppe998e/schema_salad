use std::{fmt, marker::PhantomData};

use salad_types::SaladType;
use serde::de;

use super::{IntoDeserializeSeed, SeedData};

/// A list helper deserializer, which allows flexible deserialization of data
/// represented either as maps or sequences of objects.
///
/// This is particularly useful when dealing with configurations or data formats
/// that might represent the same logical structure in different ways.
/// For example, in YAML:
///
/// ```yaml
/// # Format 1: Sequence of maps with explicit keys
/// entries:
///   - key1: value1
///     key2: value2
///
/// # Format 2: Nested map structure with the first value acting as a key
/// entries:
///   value1:
///     key2: value2
///
/// # Format 3: Map structure with key-predicate pairs
/// # Where:
/// #   - The map key becomes the value for the specified `key` field
/// #   - The map value becomes the value for the specified `predicate` field
/// entries:
///   value1: value2
/// ```
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
    /// Creates a new [`MapDeserializeSeed`] with the specified key, optional predicate, and seed data.
    ///
    /// # Arguments
    ///
    /// * `key` - The field name that will be used as the key in the deserialized structure
    /// * `pred` - Optional predicate field name. When provided, enables the simpler key-value mapping format
    /// * `data` - Additional seed data needed for deserialization
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::de;
    /// # use crate::de::{MapDeserializeSeed, SeedData};
    /// # let data = SeedData;
    /// // For mapping with explicit keys and values
    /// let seed = MapDeserializeSeed::new("class", Some("key"), &data);
    ///
    /// // For mapping with just keys (no predicate)
    /// let seed = MapDeserializeSeed::new("class", None, &data);
    /// ```
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
            key: &'static str,
            pred: Option<&'static str>,
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
                use serde::__private::de::{Content, ContentDeserializer};

                let capacity = map.size_hint().unwrap_or(1);
                let mut entries = Vec::with_capacity(capacity);

                while let Some((key, value)) = map.next_entry::<Content<'de>, Content<'de>>()? {
                    let value = match (value, self.pred) {
                        (Content::Map(mut value_map), _) => {
                            let key_field = Content::Str(self.key);
                            value_map.reserve_exact(1);
                            value_map.push((key_field, key));
                            value_map
                        }
                        (value, Some(pred)) => {
                            let key_field = Content::Str(self.key);
                            let predicate_field = Content::Str(pred);
                            vec![(key_field, key), (predicate_field, value)]
                        }
                        (_, None) => {
                            return Err(de::Error::custom(format_args!(
                                "field '{}' requires a map or predicate value",
                                self.key
                            )));
                        }
                    };

                    entries.push({
                        let deserializer = ContentDeserializer::new(Content::Map(value));
                        de::DeserializeSeed::deserialize(
                            T::deserialize_seed(self.data),
                            deserializer,
                        )?
                    });
                }

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
            key: self.key,
            pred: self.pred,
            data: self.data,
            _phant: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use salad_types::{SaladAny, SaladObject};
    use serde::__private::de::{Content, ContentDeserializer};

    use super::*;

    #[test]
    fn list_entries() {
        let input = r#"
            - class: class_1
              key: value_1
            - class: class_2
              key: value_2
            - class: class_3
              key: value_3
        "#;

        let deserializer: ContentDeserializer<'_, serde_yml::Error> = {
            let content: Content<'static> = serde_yml::from_str::<Content>(input).unwrap();
            ContentDeserializer::new(content)
        };

        let to_match = SaladAny::String("value_2".into());
        let object_list = de::DeserializeSeed::deserialize(
            MapDeserializeSeed::<'_, SaladObject>::new("class", Some("key"), &SeedData),
            deserializer,
        );

        assert!(object_list.is_ok_and(|r| matches!(r[1].get("key"), Some(s) if s == &to_match)))
    }

    #[test]
    fn map_entries() {
        let input = r#"
            class_1:
                key: value_1
            class_2:
                key: value_2
            class_3:
                key: value_3
        "#;

        let deserializer: ContentDeserializer<'_, serde_yml::Error> = {
            let content: Content<'static> = serde_yml::from_str::<Content>(input).unwrap();
            ContentDeserializer::new(content)
        };

        let to_match = SaladAny::String("value_2".into());
        let object_list = de::DeserializeSeed::deserialize(
            MapDeserializeSeed::<'_, SaladObject>::new("class", Some("key"), &SeedData),
            deserializer,
        );

        assert!(object_list.is_ok_and(|r| matches!(r[1].get("key"), Some(s) if s == &to_match)))
    }

    #[test]
    fn map_entries_with_predicate() {
        let input = r#"
            class_1: value_1
            class_2: value_2
            class_3:
                key: value_3
        "#;

        let deserializer: ContentDeserializer<'_, serde_yml::Error> = {
            let content: Content<'static> = serde_yml::from_str::<Content>(input).unwrap();
            ContentDeserializer::new(content)
        };

        let to_match = SaladAny::String("value_2".into());
        let object_list = de::DeserializeSeed::deserialize(
            MapDeserializeSeed::<'_, SaladObject>::new("class", Some("key"), &SeedData),
            deserializer,
        );

        assert!(object_list.is_ok_and(|r| matches!(r[1].get("key"), Some(s) if s == &to_match)))
    }
}
