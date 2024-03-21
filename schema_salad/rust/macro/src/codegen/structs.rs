use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::generate_root_de_impl;
use crate::{
    metadata::{InputStruct, SALAD_ATTR_ID},
    util::TypeExt,
};

pub(super) fn generate_struct(input: InputStruct) -> syn::Result<TokenStream2> {
    let InputStruct {
        salad_attrs,
        attrs,
        vis,
        ident,
        fields,
        seed_ident,
        value_ident,
        ..
    } = &input;

    let struct_getters = getters::generate_impl(&input);
    let value_enum = value::generate_enum(&input);
    let value_debug_impl = value::generate_debug_impl(&input);
    let value_ser_impl = value::generate_ser_impl(&input);
    let struct_root_de_impl = generate_root_de_impl(ident, seed_ident, salad_attrs);
    let seed_de_impl = if let Some(id_field) = fields
        .iter()
        .find(|f| f.salad_attrs.contains_and_is_true(SALAD_ATTR_ID))
    {
        de::generate_id_seed_de_impl(&input, id_field)?
    } else {
        de::generate_seed_de_impl(&input)?
    };

    Ok(quote! {
        #( #attrs )*
        #vis struct #ident(
            crate::util::Ref<
                ::std::collections::HashMap<
                    ::compact_str::CompactString,
                    self::#value_ident,
                    ::fxhash::FxBuildHasher,
                >,
            >,
        );

        #[doc(hidden)]
        pub(crate) struct #seed_ident<'_sd>(&'_sd crate::util::de::SeedData);

        #value_enum

        #[doc(hidden)]
        const _: () = {
            extern crate compact_str as _compact_str;
            extern crate fxhash as _fxhash;
            extern crate serde as _serde;
            extern crate std as _std;

            impl crate::core::SaladType for self::#ident {}

            #struct_getters

            #[automatically_derived]
            impl _serde::ser::Serialize for self::#ident {
                #[inline]
                fn serialize<S: _serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    self.0.serialize(serializer)
                }
            }

            #value_ser_impl

            #[automatically_derived]
            impl<'_de, '_sd> crate::util::de::IntoDeserializeSeed<'_de, '_sd> for #ident {
                type Value = self::#seed_ident<'_sd>;

                #[inline]
                fn into_dseed(data: &'_sd crate::util::de::SeedData) -> Self::Value {
                    #seed_ident(data)
                }
            }

            #seed_de_impl
            #struct_root_de_impl

            #value_debug_impl
        };
    })
}

mod getters {
    use super::*;

    pub(super) fn generate_impl(input: &InputStruct) -> TokenStream2 {
        let InputStruct {
            ident,
            fields,
            value_ident,
            ..
        } = &input;

        let method_docs = fields.iter().map(|f| {
            let docs = f.attrs.iter().filter(|a| a.path().is_ident("doc"));
            quote! ( #( #docs )* )
        });
        let method_ident = fields.iter().map(|f| &f.ident);
        let method_return = fields.iter().map(|f| f.ty.clone().into_typeref());
        let field_literal = fields.iter().map(|f| &f.literal);

        let field_matches = fields.iter().map(|f| {
            let value_variant = &f.variant_ident;
            let (ty, is_option) = {
                let ty = f.ty.sub_type(Some("Option"));
                (ty.unwrap_or(&f.ty), ty.is_some())
            };
            let is_primitive = ty.is_salad_primitive();

            match (is_option, is_primitive) {
                (false, false) => {
                    quote!( Some(self::#value_ident::#value_variant(v)) => v, )
                }
                (false, true) => {
                    quote!( Some(self::#value_ident::#value_variant(v)) => *v, )
                }
                (true, false) => quote! {
                    Some(self::#value_ident::#value_variant(v)) => Some(v),
                    None => None,
                },
                (true, true) => quote! {
                    Some(self::#value_ident::#value_variant(v)) => Some(*v),
                    None => None,
                },
            }
        });

        quote! {
            #[automatically_derived]
            impl self::#ident {
                #(
                    #method_docs
                    pub fn #method_ident(&self) -> #method_return {
                        match self.0.get(#field_literal) {
                            #field_matches
                            // TODO Must be rechecked when doing the builder
                            // TODO To be bench against unreachable macro
                            _ => unsafe { _std::hint::unreachable_unchecked() },
                        }
                    }
                )*

                // TODO Method documentation
                pub fn get_extension(&self, key: &str) -> ::std::option::Option<&crate::core::Any> {
                    match self.0.get(key) {
                        Some(self::#value_ident::Any(v)) => Some(v),
                        _ => None,
                    }
                }
            }
        }
    }
}

mod de {
    use super::*;
    use crate::metadata::{
        PunctuatedFields, StructField, SALAD_ATTR_MAP_KEY, SALAD_ATTR_MAP_PREDICATE,
        SALAD_ATTR_SUBSCOPE,
    };

    fn mandatory_field_iter<'a>(
        fields: &'a PunctuatedFields,
    ) -> impl Iterator<Item = Option<TokenStream2>> + 'a {
        fields.iter().filter_map(|f| {
            f.ty.sub_type(Some("Option")).is_none().then(|| {
                let literal = &f.literal;
                let null_err_str = format!("the field `{literal}` is null");

                Some(quote! {
                    match struct_map.get(#literal) {
                        Some(_) => (),
                        None => return Err(_serde::de::Error::custom(#null_err_str)),
                    };
                })
            })
        })
    }

    // Generate code for structures that HAVE an identifier field
    pub(super) fn generate_id_seed_de_impl(
        input: &InputStruct,
        id_field: &StructField,
    ) -> syn::Result<TokenStream2> {
        let InputStruct {
            ident,
            fields,
            seed_ident,
            value_ident,
            ..
        } = input;

        let fields_count = fields.len();
        let expected_str = format!("a valid `{ident}` object");

        let id_field_literal = &id_field.literal;
        let mandatory_fields = mandatory_field_iter(fields);
        let match_fields = fields
            .iter()
            .map(|f| {
                let StructField {
                    salad_attrs,
                    ty,
                    literal,
                    variant_ident,
                    ..
                } = f;

                let seed_expr = {
                    let ty = ty.sub_type(Some("Option")).unwrap_or(ty);
                    let map_key = salad_attrs.get_string(SALAD_ATTR_MAP_KEY)?;
                    let map_predicate = salad_attrs.get_string(SALAD_ATTR_MAP_PREDICATE)?;

                    match (map_key, map_predicate) {
                        (Some(key), None) => {
                            quote!( crate::util::de::MapOrSeqDeserializeSeed::new(#key, None, self.0) )
                        }
                        (Some(key), Some(pred)) => {
                            quote!( crate::util::de::MapOrSeqDeserializeSeed::new(#key, Some(#pred), self.0) )
                        }
                        _ => quote!( <#ty>::into_dseed(self.0) ),
                    }
                };

                let (subscope_push, subscope_pop) = match salad_attrs.get_string(SALAD_ATTR_SUBSCOPE)? {
                    Some(subscope) => (
                        Some(quote!( self.0.push_subscope(#subscope); )),
                        Some(quote!( self.0.pop_parent_id(); )),
                    ),
                    None => (None, None),
                };

                Ok(quote! {
                    #literal => {
                        #subscope_push
                        let value = _serde::de::DeserializeSeed::deserialize(
                            #seed_expr, value_deserializer,
                        )?;
                        #subscope_pop
                        self::#value_ident::#variant_ident(value)
                    }
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            #[automatically_derived]
            impl<'_sd, '_de> _serde::de::DeserializeSeed<'_de> for #seed_ident<'_sd> {
                type Value = self::#ident;

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: _serde::de::Deserializer<'_de>,
                {
                    struct StructVisitor<'_sd>(&'_sd crate::util::de::SeedData);

                    impl<'_sd, '_de> _serde::de::Visitor<'_de> for StructVisitor<'_sd> {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut _std::fmt::Formatter) -> _std::fmt::Result {
                            f.write_str(#expected_str)
                        }

                        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                        where
                            A: _serde::de::MapAccess<'_de>,
                        {
                            let mut is_id_declared = false;

                            let content_map = {
                                let mut content_map = _std::collections::HashMap::with_capacity_and_hasher(
                                    #fields_count, _fxhash::FxBuildHasher::default()
                                );

                                while let Some(key) = map.next_key::<_compact_str::CompactString>()? {
                                    if content_map.contains_key(&key) {
                                        return Err(_serde::de::Error::custom(format_args!(
                                            "duplicate field `{}`", &key
                                        )));
                                    }

                                    let value = if key == #id_field_literal {
                                        let id = map.next_value::<_compact_str::CompactString>()?;
                                        match self.0.generate_id(id) {
                                            Ok(v) => {
                                                is_id_declared = true;
                                                _serde::__private::de::Content::<'_de>::String(v)
                                            }
                                            Err(e) => {
                                                return Err(_serde::de::Error::custom(e));
                                            }
                                        }
                                    } else {
                                        map.next_value::<_serde::__private::de::Content<'_de>>()?
                                    };

                                    content_map.insert(key, value);
                                }

                                content_map
                            };

                            let struct_map = {
                                use crate::util::de::IntoDeserializeSeed;

                                let mut struct_map = _std::collections::HashMap::with_capacity_and_hasher(
                                    #fields_count, _fxhash::FxBuildHasher::default()
                                );

                                for (key, value) in content_map.into_iter() {
                                    let value_deserializer =
                                        _serde::__private::de::ContentDeserializer::<A::Error>::new(value);

                                    let value = match key.as_str() {
                                        #( #match_fields )*
                                        _ => {
                                            let value = _serde::de::DeserializeSeed::deserialize(
                                                <crate::core::Any>::into_dseed(self.0),
                                                value_deserializer,
                                            )?;
                                            self::#value_ident::Any(value)
                                        }
                                    };

                                    struct_map.insert(key, value);
                                }

                                struct_map
                            };

                            #( #mandatory_fields )*

                            if is_id_declared {
                                self.0.pop_parent_id();
                            }

                            Ok(self::#ident(crate::util::Ref::new(struct_map)))
                        }
                    }

                    deserializer.deserialize_map(StructVisitor(self.0))
                }
            }
        })
    }

    // Generate code for structures that DO NOT HAVE an identifier field
    pub(super) fn generate_seed_de_impl(input: &InputStruct) -> syn::Result<TokenStream2> {
        let InputStruct {
            ident,
            fields,
            seed_ident,
            value_ident,
            ..
        } = input;

        let fields_count = fields.len();
        let expected_str = format!("a valid `{ident}` object");

        let mandatory_fields = mandatory_field_iter(fields);
        let match_fields = fields
            .iter()
            .map(|f| {
                let StructField {
                    salad_attrs,
                    ty,
                    literal,
                    variant_ident,
                    ..
                } = f;

                let seed_expr = {
                    let ty = ty.sub_type(Some("Option")).unwrap_or(ty);
                    let map_key = salad_attrs.get_string(SALAD_ATTR_MAP_KEY)?;
                    let map_predicate = salad_attrs.get_string(SALAD_ATTR_MAP_PREDICATE)?;

                    match (map_key, map_predicate) {
                        (Some(key), None) => {
                            quote!( crate::util::de::MapOrSeqDeserializeSeed::new(#key, None, self.0) )
                        }
                        (Some(key), Some(pred)) => {
                            quote!( crate::util::de::MapOrSeqDeserializeSeed::new(#key, Some(#pred), self.0) )
                        }
                        _ => quote!( <#ty>::into_dseed(self.0) ),
                    }
                };

                let (subscope_push, subscope_pop) = match salad_attrs.get_string(SALAD_ATTR_SUBSCOPE)? {
                    Some(subscope) => (
                        Some(quote!( self.0.push_subscope(#subscope); )),
                        Some(quote!( self.0.pop_parent_id(); )),
                    ),
                    None => (None, None),
                };

                Ok(quote! {
                    #literal => {
                        #subscope_push
                        let value = map.next_value_seed(#seed_expr)?;
                        #subscope_pop
                        self::#value_ident::#variant_ident(value)
                    }
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            #[automatically_derived]
            impl<'_sd, '_de> _serde::de::DeserializeSeed<'_de> for #seed_ident<'_sd> {
                type Value = self::#ident;

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: _serde::de::Deserializer<'_de>,
                {
                    struct StructVisitor<'_sd>(&'_sd crate::util::de::SeedData);

                    impl<'_sd, '_de> _serde::de::Visitor<'_de> for StructVisitor<'_sd> {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut _std::fmt::Formatter) -> _std::fmt::Result {
                            f.write_str(#expected_str)
                        }

                        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                        where
                            A: _serde::de::MapAccess<'_de>,
                        {
                            let struct_map = {
                                use crate::util::de::IntoDeserializeSeed;

                                let mut struct_map = _std::collections::HashMap::with_capacity_and_hasher(
                                    #fields_count, _fxhash::FxBuildHasher::default()
                                );

                                while let Some(key) = map.next_key::<_compact_str::CompactString>()? {
                                    if struct_map.contains_key(&key) {
                                        return Err(_serde::de::Error::custom(format_args!(
                                            "duplicate field `{}`",
                                            &key
                                        )));
                                    }

                                    let value = match key.as_str() {
                                        #( #match_fields )*
                                        _ => {
                                            let value = map.next_value_seed(crate::core::Any::into_dseed(self.0))?;
                                            self::#value_ident::Any(value)
                                        }
                                    };

                                    struct_map.insert(key, value);
                                }

                                struct_map
                            };

                            #( #mandatory_fields )*

                            Ok(self::#ident(crate::util::Ref::new(struct_map)))
                        }
                    }

                    deserializer.deserialize_map(StructVisitor(self.0))
                }
            }
        })
    }
}

mod value {
    use std::collections::HashSet;

    use fxhash::FxBuildHasher;

    use super::*;

    pub(super) fn generate_enum(input: &InputStruct) -> TokenStream2 {
        let InputStruct {
            fields,
            value_ident,
            ..
        } = input;

        let mut variants_seen =
            HashSet::with_capacity_and_hasher(fields.len(), FxBuildHasher::default());
        let variant_iter = fields.iter().filter_map(|f| {
            let variant_ident = &f.variant_ident;
            let ty: &syn::Type = f.ty.sub_type(Some("Option")).unwrap_or(&f.ty);
            let tokenstream = quote!( #variant_ident(#ty) );

            (variant_ident != "Any" && variants_seen.insert(variant_ident)).then_some(tokenstream)
        });

        quote! {
            #[doc(hidden)]
            #[derive(Clone)]
            enum #value_ident {
                #( #variant_iter, )*
                Any(crate::core::Any),
            }
        }
    }

    pub(super) fn generate_debug_impl(input: &InputStruct) -> TokenStream2 {
        let InputStruct {
            fields,
            value_ident,
            ..
        } = input;

        let mut variants_seen =
            HashSet::with_capacity_and_hasher(fields.len(), FxBuildHasher::default());
        let variant_ident_iter = fields.iter().filter_map(|f| {
            let variant_ident = &f.variant_ident;
            (variant_ident != "Any" && variants_seen.insert(variant_ident)).then_some(variant_ident)
        });

        quote! {
            #[automatically_derived]
            impl _std::fmt::Debug for self::#value_ident {
                fn fmt(&self, f: &mut _std::fmt::Formatter<'_>) -> _std::fmt::Result {
                    match self {
                        #( Self::#variant_ident_iter(v) => _std::fmt::Debug::fmt(v, f), )*
                        Self::Any(v) => _std::fmt::Debug::fmt(v, f),
                    }
                }
            }
        }
    }

    pub(super) fn generate_ser_impl(input: &InputStruct) -> TokenStream2 {
        let InputStruct {
            fields,
            value_ident,
            ..
        } = input;

        let mut variants_seen =
            HashSet::with_capacity_and_hasher(fields.len(), FxBuildHasher::default());
        let variant_ident_iter = fields.iter().filter_map(|f| {
            let variant_ident = &f.variant_ident;
            (variant_ident != "Any" && variants_seen.insert(variant_ident)).then_some(variant_ident)
        });

        quote! {
            #[automatically_derived]
            impl _serde::ser::Serialize for self::#value_ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: _serde::ser::Serializer,
                {
                    match self {
                        #( Self::#variant_ident_iter(v) => v.serialize(serializer), )*
                        Self::Any(v) => v.serialize(serializer),
                    }
                }
            }
        }
    }
}
