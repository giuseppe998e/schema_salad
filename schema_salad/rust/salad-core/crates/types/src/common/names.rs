macro_rules! string_match_struct {
    (
        $(
            $( #[$attrs:meta] )*
            $value:literal => $ident:ident
        ),* $(,)?
    ) => {
        $(
            $( #[$attrs] )*
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $ident;

            impl crate::SaladType for $ident {}

            impl core::fmt::Display for $ident {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str($value)
                }
            }

            impl serde::ser::Serialize for $ident {
                #[inline]
                fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::ser::Serializer,
                {
                    s.serialize_str($value)
                }
            }

            impl<'de> serde::de::Deserialize<'de> for $ident {
                fn deserialize<D>(d: D) -> Result<Self, D::Error>
                where
                    D: serde::de::Deserializer<'de>,
                {
                    struct NameVisitor;

                    impl serde::de::Visitor<'_> for NameVisitor {
                        type Value = $ident;

                        fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                            f.write_str(concat!("the string '", $value, '\''))
                        }

                        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            match v {
                                $value => Ok($ident),
                                _ => Err(serde::de::Error::invalid_value(
                                    serde::de::Unexpected::Str(v),
                                    &$value,
                                )),
                            }
                        }

                        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            const VALUE_BYTES: &[u8] = $value.as_bytes();

                            match v {
                                VALUE_BYTES => Ok($ident),
                                _ => Err(serde::de::Error::invalid_value(
                                    serde::de::Unexpected::Bytes(v),
                                    &$value,
                                )),
                            }
                        }
                    }

                    d.deserialize_str(NameVisitor)
                }
            }

        )*
    };
}

string_match_struct! {
    /// Matches constant value `array`.
    "array" => ArrayName,

    /// Matches constant value `enum`.
    "enum" => EnumName,

    /// Matches constant value `record`.
    "record" => RecordName,
}
