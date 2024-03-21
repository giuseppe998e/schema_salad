use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::metadata::{InputUnit, SALAD_ATTR_AS_STR};

pub(super) fn generate_unit(input: InputUnit) -> syn::Result<TokenStream2> {
    let InputUnit {
        salad_attrs,
        attrs,
        vis,
        ident,
    } = &input;

    let (value, expected_str) = match salad_attrs.get_string(SALAD_ATTR_AS_STR)? {
        Some(value) => (value, format!("a string of value \"{}\"", value.value())),
        None => {
            return Err(syn::Error::new(
                ident.span(),
                format_args!(
                    "Attribute '#[salad({} = \"...\")' missing, unable to generate the type.",
                    SALAD_ATTR_AS_STR,
                ),
            ))
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis struct #ident;

        #[doc(hidden)]
        const _: () = {
            extern crate serde as _serde;
            extern crate std as _std;

            #[automatically_derived]
            impl crate::core::SaladType for self::#ident {}

            #[automatically_derived]
            impl _std::convert::TryFrom<&str> for self::#ident {
                type Error = ();

                #[inline]
                fn try_from(value: &str) -> Result<Self, Self::Error> {
                    <self::#ident as _std::str::FromStr>::from_str(value)
                }
            }

            #[automatically_derived]
            impl _std::str::FromStr for self::#ident {
                type Err = ();

                fn from_str(input: &str) -> Result<Self, Self::Err> {
                    match input {
                        #value => Ok(Self),
                        _ => Err(())
                    }
                }
            }

            #[automatically_derived]
            impl _std::fmt::Display for self::#ident {
                #[inline]
                fn fmt(&self, f: &mut _std::fmt::Formatter<'_>) -> _std::fmt::Result {
                    f.write_str(#value)
                }
            }

            #[automatically_derived]
            impl _serde::Serialize for self::#ident {
                #[inline]
                fn serialize<S: _serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    serializer.collect_str(#value)
                }
            }

            #[automatically_derived]
            impl<'_de, '_sd> crate::util::de::IntoDeserializeSeed<'_de, '_sd> for self::#ident {
                type Value = _std::marker::PhantomData<Self>;

                #[inline]
                fn into_dseed(_: &'_sd crate::util::de::SeedData) -> Self::Value {
                    _std::marker::PhantomData
                }
            }

            #[automatically_derived]
            impl<'_de> _serde::Deserialize<'_de> for self::#ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: _serde::Deserializer<'_de>,
                {
                    struct UnitVisitor;

                    impl<'de> _serde::de::Visitor<'de> for UnitVisitor {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut _std::fmt::Formatter) -> _std::fmt::Result {
                            f.write_str(#expected_str)
                        }

                        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                        where
                            E: _serde::de::Error,
                        {
                            match v {
                                #value => Ok(self::#ident),
                                _ => Err(_serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Str(v),
                                    &#value,
                                )),
                            }
                        }
                    }

                    deserializer.deserialize_str(UnitVisitor)
                }
            }
        };
    })
}
