use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::metadata::{InputEnum, SALAD_ATTR_AS_STR};

pub(super) fn generate_enum(input: InputEnum) -> syn::Result<TokenStream2> {
    let InputEnum {
        ident, variants, ..
    } = &input;

    let variant_strings = variants
        .iter()
        .filter_map(|v| {
            if v.field.is_none() {
                match v.salad_attrs.get_string(SALAD_ATTR_AS_STR) {
                    Ok(Some(value)) => Some(Ok(value)),
                    Ok(None) => Some(Err(syn::Error::new(
                        ident.span(),
                        format_args!(
                            "Attribute '#[salad({} = \"...\")' missing for variant `{}`, unable to generate the type.",
                            SALAD_ATTR_AS_STR,
                            v.ident,
                        ),
                    ))),
                    Err(e) => Some(Err(e)),
                }
            } else {
                None
            }
        })
    .collect::<syn::Result<Vec<_>>>()?;

    match (variants.len(), variant_strings.len()) {
        (1.., 0) => Ok(tuples::generate_enum(&input)),
        (v @ 1.., u @ 1..) if v == u => Ok(units::generate_enum(&input, &variant_strings)),
        (1.., 1..) => Err(syn::Error::new(
            ident.span(),
            "Mixed tuple and unit enum is not supported.",
        )),
        _ => Err(syn::Error::new(
            ident.span(),
            "Enum without variants is not supported",
        )),
    }
}

mod tuples {
    use super::*;
    use crate::{metadata::SALAD_ATTR_ROOT, util::TypeExt};

    pub(super) fn generate_enum(input: &InputEnum) -> TokenStream2 {
        let InputEnum {
            attrs,
            vis,
            ident,
            variants,
            seed_ident,
            ..
        } = &input;

        let tryfrom_impls = self::generate_from_impls(input);
        let serialize_impl = self::generate_ser_impl(input);
        let deserialize_impl = self::generate_de_impl(input);
        let root_deserialize_impl = self::generate_root_de_impl(input);

        quote! {
            #(#attrs)*
            #vis enum #ident {
                #variants
            }

            #[doc(hidden)]
            pub(crate) struct #seed_ident<'_sd>(&'_sd crate::de::SeedData);

            #[doc(hidden)]
            const _: () = {
                extern crate serde as _serde;
                extern crate std as _std;

                #[automatically_derived]
                impl crate::core::SaladType for self::#ident {}

                #( #tryfrom_impls )*

                #serialize_impl

                #[automatically_derived]
                impl<'_de, '_sd> crate::de::IntoDeserializeSeed<'_de, '_sd> for self::#ident {
                    type Value = self::#seed_ident<'_sd>;

                    #[inline]
                    fn into_dseed(data: &'_sd crate::de::SeedData) -> Self::Value {
                        self::#seed_ident(data)
                    }
                }

                #deserialize_impl
                #root_deserialize_impl
            };
        }
    }

    // TODO Fix "From" generation in case of "Box<[...]>"
    fn generate_from_impls<'i>(input: &'i InputEnum) -> impl Iterator<Item = TokenStream2> + 'i {
        let InputEnum {
            ident, variants, ..
        } = input;

        variants.iter().map(move |v| {
            let variant_ident = &v.ident;
            let variant_ty = {
                debug_assert!(v.field.is_some());
                unsafe { &v.field.as_ref().unwrap_unchecked().ty }
            };

            let variant_ty_ident = variant_ty.to_variant_ident().to_string();
            match variant_ty_ident.as_str() {
                "StrValue" => quote! {
                    #[automatically_derived]
                    impl _std::convert::From<&str> for self::#ident {
                        fn from(value: &str) -> Self {
                            self::#ident::#variant_ident(
                                _std::convert::Into::into(value)
                            )
                        }
                    }
                },
                "BoxStrValueSlice" => quote! {
                    #[automatically_derived]
                    impl _std::convert::From<&[&str]> for self::#ident {
                        fn from(value: &[&str]) -> Self {
                            let box_slice = value
                                    .iter()
                                    .map(|s| _std::convert::Into::into(*s))
                                    .collect::<Box<[_]>>();
                            self::#ident::#variant_ident(box_slice)
                        }
                    }
                },
                _ => quote! {
                    #[automatically_derived]
                    impl _std::convert::From<#variant_ty> for self::#ident {
                        fn from(value: #variant_ty) -> Self {
                            self::#ident::#variant_ident(value)
                        }
                    }
                },
            }
        })
    }

    fn generate_ser_impl(input: &InputEnum) -> TokenStream2 {
        let InputEnum {
            ident, variants, ..
        } = input;

        let variant_ident_iter = variants.iter().map(|v| &v.ident);

        quote! {
            #[automatically_derived]
            impl _serde::ser::Serialize for self::#ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: _serde::ser::Serializer,
                {
                    match self {
                        #( Self::#variant_ident_iter(v) => v.serialize(serializer) ),*
                    }
                }
            }
        }
    }

    fn generate_de_impl(input: &InputEnum) -> TokenStream2 {
        let InputEnum {
            ident,
            variants,
            seed_ident,
            ..
        } = input;

        let err_string = format!("data did not match any variant of enum `{}`", ident);

        let variant_ident_iter = variants.iter().map(|v| &v.ident);
        let variant_ty_iter = variants
            .iter()
            .map(|v| {
                debug_assert!(v.field.is_some());
                unsafe { &v.field.as_ref().unwrap_unchecked().ty }
            });

        quote! {
            #[automatically_derived]
            impl<'_de, '_sd> _serde::de::DeserializeSeed<'_de> for self::#seed_ident<'_sd> {
                type Value = self::#ident;

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: _serde::de::Deserializer<'_de>,
                {
                    let content =
                        <_serde::__private::de::Content<'_de> as _serde::Deserialize>::deserialize(
                            deserializer,
                        )?;
                    let content_deserializer =
                        _serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content);

                    #(
                        {
                            let data = self.0.clone();
                            let deserialize_seed =
                                <#variant_ty_iter as crate::de::IntoDeserializeSeed>::into_dseed(
                                    &data
                                );

                            if let Ok(v) =
                                _serde::de::DeserializeSeed::deserialize(deserialize_seed, content_deserializer)
                            {
                                return match self.0.extend(data) {
                                    Ok(_) => Ok(self::#ident::#variant_ident_iter(v)),
                                    Err(e) => Err(_serde::de::Error::custom(e)),
                                };
                            }
                        }
                    )*

                    Err(_serde::de::Error::custom(#err_string))
                }
            }
        }
    }

    fn generate_root_de_impl(input: &InputEnum) -> Option<TokenStream2> {
        let InputEnum {
            salad_attrs,
            ident,
            variants,
            ..
        } = input;

        if !salad_attrs.contains_and_is_true(SALAD_ATTR_ROOT) {
            return None;
        }

        let err_string = format!("data did not match any variant of enum `{}`", ident);

        let variant_ident_iter = variants.iter().map(|v| &v.ident);
        let variant_ty_iter = variants
            .iter()
            .map(|v| {
                debug_assert!(v.field.is_some());
                unsafe { &v.field.as_ref().unwrap_unchecked().ty }
            });

        Some(quote! {
            #[automatically_derived]
            impl<'_de> _serde::de::Deserialize<'_de> for self::#ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: _serde::de::Deserializer<'_de>
                {
                    let content =
                    <_serde::__private::de::Content<'_de> as _serde::Deserialize>::deserialize(
                        deserializer,
                    )?;
                    let content_deserializer =
                        _serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content);

                    #(
                        {
                            let data = crate::de::SeedData::new();
                            let deserialize_seed =
                                <#variant_ty_iter as crate::de::IntoDeserializeSeed>::into_dseed(
                                    &data
                                );

                            if let Ok(v) =
                                _serde::de::DeserializeSeed::deserialize(deserialize_seed, content_deserializer)
                            {
                                return Ok(self::#ident::#variant_ident_iter(v));
                            }
                        }
                    )*

                    Err(_serde::de::Error::custom(#err_string))
                }
            }
        })
    }
}

mod units {
    use syn::LitStr;

    use super::*;
    use crate::codegen::generate_root_de_impl;

    pub(super) fn generate_enum(input: &InputEnum, variant_strings: &[&LitStr]) -> TokenStream2 {
        let InputEnum {
            salad_attrs,
            attrs,
            vis,
            ident,
            variants,
            seed_ident,
        } = input;

        let root_deserialize_impl = generate_root_de_impl(ident, seed_ident, salad_attrs);

        let variant_ident_iter1 = variants.iter().map(|v| &v.ident);
        let variant_ident_iter2 = variant_ident_iter1.clone();

        let variant_value_iter1 = variant_strings.iter().map(|s| s);
        let variant_value_iter2 = variant_value_iter1.clone();

        let values_str = variant_value_iter2
            .clone()
            .map(|s| format!("\"{}\"", s.value()))
            .collect::<Vec<_>>()
            .join(", ");

        let expected_str = {
            let mut expected_str = "a string with one of the following values: ".to_owned();
            expected_str.push_str(&values_str);
            expected_str.push('.');
            expected_str
        };

        quote! {
            #(#attrs)*
            #vis enum #ident {
                #variants
            }

            #[doc(hidden)]
            const _: () = {
                extern crate serde as _serde;
                extern crate std as _std;

                #[automatically_derived]
                impl crate::core::SaladType for self::#ident {}

                #[automatically_derived]
                impl _std::convert::TryFrom<&str> for self::#ident {
                    type Error = ();

                    fn try_from(value: &str) -> Result<Self, Self::Error> {
                        <self::#ident as _std::str::FromStr>::from_str(value)
                    }
                }

                #[automatically_derived]
                impl _std::str::FromStr for self::#ident {
                    type Err = ();

                    fn from_str(input: &str) -> Result<Self, Self::Err> {
                        match input {
                            #( #variant_value_iter1 => Ok(self::#ident::#variant_ident_iter1), )*
                            _ => Err(()),
                        }
                    }
                }

                #[automatically_derived]
                impl _std::fmt::Display for self::#ident {
                    fn fmt(&self, f: &mut _std::fmt::Formatter<'_>) -> _std::fmt::Result {
                        match self {
                            #( Self::#variant_ident_iter2 => f.write_str(#variant_value_iter2) ),*
                        }
                    }
                }

                #[automatically_derived]
                impl _serde::Serialize for self::#ident {
                    fn serialize<S: _serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        serializer.collect_str(self)
                    }
                }

                #[automatically_derived]
                impl<'_de, '_sd> crate::de::IntoDeserializeSeed<'_de, '_sd> for self::#ident {
                    type Value = _std::marker::PhantomData<Self>;

                    #[inline]
                    fn into_dseed(_: &'_sd crate::de::SeedData) -> Self::Value {
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

                            fn visit_str<E: _serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                                <self::#ident as _std::str::FromStr>::from_str(v)
                                    .map_err(|_| _serde::de::Error::invalid_value(
                                        _serde::de::Unexpected::Str(v),
                                        &#values_str,
                                    ))
                            }
                        }

                        deserializer.deserialize_str(UnitVisitor)
                    }
                }

                #root_deserialize_impl
            };
        }
    }
}
