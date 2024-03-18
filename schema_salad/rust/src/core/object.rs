use std::{collections::HashMap, fmt};

use compact_str::CompactString;
use fxhash::FxBuildHasher;
use serde::{de, ser};

use crate::{
    core::{Any, SaladType},
    util::de::{IntoDeserializeSeed, SeedData},
};

#[derive(Debug, Clone)]
pub struct Object(HashMap<CompactString, Any, FxBuildHasher>);

impl Object {
    pub fn get<'v>(&'v self, key: &str) -> Option<&'v Any> {
        self.0.get(key)
    }
}

impl SaladType for Object {}

impl ser::Serialize for Object {
    #[inline]
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for Object {
    type Value = ObjectSeed<'sd>;

    #[inline]
    fn into_dseed(data: &'sd SeedData) -> Self::Value {
        ObjectSeed(data)
    }
}

pub(crate) struct ObjectSeed<'sd>(&'sd SeedData);

impl<'sd, 'de> de::DeserializeSeed<'de> for ObjectSeed<'sd> {
    type Value = Object;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ObjectVisitor<'s>(&'s SeedData);

        impl<'s, 'de> de::Visitor<'de> for ObjectVisitor<'s> {
            type Value = Object;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a key-value object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let fields_count = map.size_hint().unwrap_or(0);
                let mut fxmap =
                    HashMap::with_capacity_and_hasher(fields_count, FxBuildHasher::default());

                while let Some(key) = map.next_key::<CompactString>()? {
                    if fxmap.contains_key(&key) {
                        return Err(de::Error::custom(format_args!(
                            "duplicate field `{}`",
                            &key
                        )));
                    }

                    //let value = match key.as_ref() {
                    //    // The "id" field must be unique throughout the document
                    //    "id" => {
                    //        let value = map.next_value::<StrValue>()?;
                    //        if !self.0.validate_id(value.as_str()) {
                    //            return Err(de::Error::custom(format_args!(
                    //                "duplicate identifier: \"{value}\""
                    //            )));
                    //        }
                    //        Any::String(value)
                    //    }
                    //    _ => map.next_value_seed(Any::into_dseed(self.0))?,
                    //};

                    let value = map.next_value_seed(Any::into_dseed(self.0))?;
                    fxmap.insert(key, value);
                }

                Ok(Object(fxmap))
            }
        }

        deserializer.deserialize_map(ObjectVisitor(self.0))
    }
}
