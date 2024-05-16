use std::marker::PhantomData;

use serde::{__private::de as private_de, de};

/// Wraps around a Serde's MapAccess, providing the ability
/// to peek at the next key without consuming it.
pub struct PeekableMapAccess<'de, A> {
    map: A,
    peeked: Option<Option<private_de::Content<'de>>>,
}

impl<'de, A> PeekableMapAccess<'de, A>
where
    A: de::MapAccess<'de>,
{
    /// Creates a new `PeekableMapAccess` from the given Serde's MapAccess.
    pub fn new(map: A) -> Self {
        Self { map, peeked: None }
    }

    /// Peeks at the next key in the map without consuming it.
    pub fn peek_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, A::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let key_ref = match self.peeked.as_ref() {
            Some(key_ref) => key_ref,
            None => {
                self.peeked = Some(self.map.next_key::<private_de::Content<'de>>()?);

                // SAFETY: a `None` variant for `self` would have been replaced by a `Some`
                // variant in the code above.
                unsafe { self.peeked.as_ref().unwrap_unchecked() }
            }
        };

        match key_ref {
            Some(key_ref) => {
                let deserializer = private_de::ContentRefDeserializer::new(key_ref);
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    /// Peeks at the next key in the map without consuming it.
    ///
    /// This method exists as a convenience for `Deserialize` implementations.
    #[inline]
    pub fn peek_key<K>(&mut self) -> Result<Option<K>, A::Error>
    where
        K: de::Deserialize<'de>,
    {
        self.peek_key_seed(PhantomData)
    }
}

impl<'de, A> de::MapAccess<'de> for PeekableMapAccess<'de, A>
where
    A: de::MapAccess<'de>,
{
    type Error = A::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.peeked.take() {
            None => self.map.next_key_seed(seed),
            Some(Some(key)) => {
                let deserializer = private_de::ContentDeserializer::new(key);
                seed.deserialize(deserializer).map(Some)
            }
            Some(None) => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        //self.peeked = None; // Clears the previous peeked key
        self.map.next_value_seed(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.map.size_hint()
    }
}
