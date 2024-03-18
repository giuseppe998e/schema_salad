use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, LitStr};

use crate::{
    codegen::generate_root_de_impl,
    metadata::{EnumVariant, InputEnum, SALAD_ATTR_AS_STR},
};

pub(super) fn generate_enum(input: InputEnum) -> syn::Result<TokenStream2> {
    let InputEnum {
        ident, variants, ..
    } = &input;

    // TODO Remove vector allocations
    let variants_len = variants.len();
    let mut unnamed_variants = Vec::with_capacity(variants_len);
    let mut named_variants = Vec::with_capacity(variants_len);

    for variant in variants {
        if variant.field.is_some() {
            named_variants.push(variant);
        } else {
            match variant.salad_attrs.get_string(SALAD_ATTR_AS_STR)? {
                Some(value) => unnamed_variants.push((variant, value)),
                None => return Err(syn::Error::new(
                    ident.span(),
                    format_args!(
                        "Attribute '#[salad({} = \"...\")' missing for {}, unable to generate the type.",
                        SALAD_ATTR_AS_STR,
                        variant.ident,
                    ),
                ))
            }
        }
    }

    match (named_variants.is_empty(), unnamed_variants.is_empty()) {
        (false, true) => Ok(named::generate_enum(&input, &named_variants)),
        (true, false) => Ok(unnamed::generate_enum(&input, &unnamed_variants)),
        _ => Err(syn::Error::new(
            input.ident.span(),
            "Mixed named and unnamed variant enums are not supported.",
        )),
    }
}

mod unnamed {
    use super::*;

    pub(super) fn generate_enum(
        input: &InputEnum,
        unnamed_variants: &[(&EnumVariant, &LitStr)],
    ) -> TokenStream2 {
        let InputEnum {
            salad_attrs,
            attrs,
            vis,
            ident,
            variants,
            seed_ident,
        } = input;

        let root_deserialize_impl = generate_root_de_impl(ident, seed_ident, salad_attrs);

        let variant_ident_iter1 = unnamed_variants.iter().map(|(v, _)| &v.ident);
        let variant_ident_iter2 = variant_ident_iter1.clone();
        let variant_ident_iter3 = variant_ident_iter1.clone();

        let variant_value_iter1 = unnamed_variants.iter().map(|(_, s)| s);
        let variant_value_iter2 = variant_value_iter1.clone();
        let variant_value_iter3 = variant_value_iter1.clone();

        let values_str = {
            let values_str = variant_value_iter1
                .clone()
                .map(|s| format!("\"{}\"", s.value()))
                .collect::<Vec<_>>();
            values_str.join(", ")
        };

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
                impl crate::core::SaladType for #ident {}

                #[automatically_derived]
                impl _std::fmt::Display for #ident {
                    #[inline]
                    fn fmt(&self, f: &mut _std::fmt::Formatter<'_>) -> _std::fmt::Result {
                        match self {
                            #( Self::#variant_ident_iter1 => f.write_str(#variant_value_iter1) ),*
                        }
                    }
                }

                #[automatically_derived]
                impl _serde::Serialize for #ident {
                    fn serialize<S: _serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        match self {
                            #( Self::#variant_ident_iter2 => serializer.collect_str(#variant_value_iter2) ),*
                        }
                    }
                }

                #root_deserialize_impl

                #[automatically_derived]
                impl<'_de, '_sd> crate::util::de::IntoDeserializeSeed<'_de, '_sd> for #ident {
                    type Value = _std::marker::PhantomData<Self>;

                    #[inline]
                    fn into_dseed(_: &'_sd crate::util::de::SeedData) -> Self::Value {
                        _std::marker::PhantomData
                    }
                }

                #[automatically_derived]
                impl<'_de> _serde::Deserialize<'_de> for #ident {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: _serde::Deserializer<'_de>,
                    {
                        struct UnitVisitor;

                        impl<'de> _serde::de::Visitor<'de> for UnitVisitor {
                            type Value = #ident;

                            fn expecting(&self, f: &mut _std::fmt::Formatter) -> _std::fmt::Result {
                                f.write_str(#expected_str)
                            }

                            fn visit_str<E: _serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                                match v {
                                    #( #variant_value_iter3 => Ok(#ident::#variant_ident_iter3), )*
                                    _ => Err(_serde::de::Error::invalid_value(
                                        _serde::de::Unexpected::Str(v),
                                        &#values_str,
                                    ))
                                }
                            }
                        }

                        deserializer.deserialize_str(UnitVisitor)
                    }
                }
            };
        }
    }
}

mod named {
    use super::*;
    use crate::metadata::{MacroAttributes, SALAD_ATTR_ROOT};

    pub(super) fn generate_enum(
        input: &InputEnum,
        named_variants: &[&EnumVariant],
    ) -> TokenStream2 {
        let InputEnum {
            salad_attrs,
            attrs,
            vis,
            ident,
            variants,
            seed_ident,
        } = &input;

        let serialize_impl = generate_ser_impl(ident, named_variants);
        let root_deserialize_impl = self::generate_root_de_impl(ident, salad_attrs, named_variants);
        let deserialize_impl = generate_de_impl(ident, seed_ident, named_variants);

        quote! {
            #(#attrs)*
            #vis enum #ident {
                #variants
            }

            #[doc(hidden)]
            pub(crate) struct #seed_ident<'_sd>(&'_sd crate::util::de::SeedData);

            #[doc(hidden)]
            const _: () = {
                extern crate serde as _serde;
                extern crate std as _std;

                #[automatically_derived]
                impl crate::core::SaladType for #ident {}

                #serialize_impl

                #[automatically_derived]
                impl<'_de, '_sd> crate::util::de::IntoDeserializeSeed<'_de, '_sd> for #ident {
                    type Value = #seed_ident<'_sd>;

                    #[inline]
                    fn into_dseed(data: &'_sd crate::util::de::SeedData) -> Self::Value {
                        #seed_ident(data)
                    }
                }

                #deserialize_impl
                #root_deserialize_impl
            };
        }
    }

    fn generate_ser_impl(ident: &Ident, named_variants: &[&EnumVariant]) -> TokenStream2 {
        let variant_ident_iter = named_variants.iter().map(|v| &v.ident);

        quote! {
            #[automatically_derived]
            impl _serde::ser::Serialize for #ident {
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

    fn generate_root_de_impl(
        ident: &Ident,
        salad_attrs: &MacroAttributes,
        named_variants: &[&EnumVariant],
    ) -> Option<TokenStream2> {
        if !salad_attrs.contains_and_is_true(SALAD_ATTR_ROOT) {
            return None;
        }
        let err_string = format!("data did not match any variant of enum `{}`", ident);
        let variant_ident_iter = named_variants.iter().map(|v| &v.ident);
        let variant_ty_iter = named_variants
            .iter()
            .map(|v| unsafe { &v.field.as_ref().unwrap_unchecked().ty });

        Some(quote! {
            #[automatically_derived]
            impl<'_de> _serde::de::Deserialize<'_de> for #ident {
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
                            let data = crate::util::de::SeedData::new();
                            let deserialize_seed =
                                <#variant_ty_iter as crate::util::de::IntoDeserializeSeed>::into_dseed(
                                    &data
                                );

                            if let Ok(v) =
                                _serde::de::DeserializeSeed::deserialize(deserialize_seed, content_deserializer)
                            {
                                return Ok(#ident::#variant_ident_iter(v));
                            }
                        }
                    )*

                    Err(_serde::de::Error::custom(#err_string))
                }
            }
        })
    }

    fn generate_de_impl(
        ident: &Ident,
        seed_ident: &Ident,
        named_variants: &[&EnumVariant],
    ) -> TokenStream2 {
        let err_string = format!("data did not match any variant of enum `{}`", ident);
        let variant_ident_iter = named_variants.iter().map(|v| &v.ident);
        let variant_ty_iter = named_variants
            .iter()
            .map(|v| unsafe { &v.field.as_ref().unwrap_unchecked().ty });

        quote! {
            #[automatically_derived]
            impl<'_de, '_sd> _serde::de::DeserializeSeed<'_de> for #seed_ident<'_sd> {
                type Value = #ident;

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
                                <#variant_ty_iter as crate::util::de::IntoDeserializeSeed>::into_dseed(
                                    &data
                                );

                            if let Ok(v) =
                                _serde::de::DeserializeSeed::deserialize(deserialize_seed, content_deserializer)
                            {
                                return match self.0.extend(data) {
                                    Ok(_) => Ok(#ident::#variant_ident_iter(v)),
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
}
