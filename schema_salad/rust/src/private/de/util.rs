use std::{marker::PhantomData, vec};

use serde::{__private::de as private_de, de};

pub(crate) struct VecMapAccess<'de, E> {
    values: vec::IntoIter<(private_de::Content<'de>, private_de::Content<'de>)>,
    next_value: Option<private_de::Content<'de>>,
    _phant_err: PhantomData<E>,
}

impl<'de, E> From<Vec<(private_de::Content<'de>, private_de::Content<'de>)>>
    for VecMapAccess<'de, E>
where
    E: de::Error,
{
    fn from(value: Vec<(private_de::Content<'de>, private_de::Content<'de>)>) -> Self {
        Self {
            values: value.into_iter(),
            next_value: None,
            _phant_err: PhantomData,
        }
    }
}

impl<'de, E> de::MapAccess<'de> for VecMapAccess<'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.values.next() {
            Some((key, value)) => {
                // Cache the value
                self.next_value = Some(value);

                // Deserialize the key
                let deserializer = private_de::ContentDeserializer::new(key);
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.next_value.take() {
            Some(v) => {
                let deserializer = private_de::ContentDeserializer::new(v);
                seed.deserialize(deserializer)
            }
            None => Err(Self::Error::custom("no cached values in map")),
        }
    }
}
