use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::generate_root_de_impl;
use crate::{
    metadata::{InputStruct, SALAD_ATTR_DEFAULT, SALAD_ATTR_ID},
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

    let struct_getters = getters::generate_impl(&input)?;
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
            crate::__private::Ref<
                ::std::collections::HashMap<
                    ::compact_str::CompactString,
                    self::#value_ident,
                    ::fxhash::FxBuildHasher,
                >,
            >,
        );

        #[doc(hidden)]
        pub(crate) struct #seed_ident<'_sd>(&'_sd crate::__private::de::SeedData);

        #value_enum

        #[doc(hidden)]
        const _: () = {
            extern crate compact_str as _compact_str;
            extern crate fxhash as _fxhash;
            extern crate serde as _serde;
            extern crate std as _std;

            #[automatically_derived]
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
            impl<'_de, '_sd> crate::__private::de::IntoDeserializeSeed<'_de, '_sd> for #ident {
                type Value = self::#seed_ident<'_sd>;

                #[inline]
                fn into_dseed(data: &'_sd crate::__private::de::SeedData) -> Self::Value {
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

    pub(super) fn generate_impl(input: &InputStruct) -> syn::Result<TokenStream2> {
        let InputStruct {
            salad_attrs,
            ident,
            fields,
            value_ident,
            ..
        } = &input;

        let method_ident = fields.iter().map(|f| &f.ident);
        let method_docs = fields.iter().map(|f| {
            let docs = f.attrs.iter().filter(|a| a.path().is_ident("doc"));
            quote! ( #( #docs )* )
        });
        let method_return = fields.iter().map(|f| {
            if salad_attrs.contains(SALAD_ATTR_DEFAULT) {
                let ty = f.ty.sub_type(Some("Option")).unwrap_or(&f.ty).clone();
                ty.into_typeref()
            } else {
                f.ty.clone().into_typeref()
            }
        });

        let field_literal = fields.iter().map(|f| &f.literal);
        let field_matches = fields.iter().map(|f| {
            let value_variant = &f.variant_ident;
            let (ty, optional) = {
                let ty = f.ty.sub_type(Some("Option"));
                (ty.unwrap_or(&f.ty), ty.is_some())
            };
            let primitive = ty.is_salad_primitive();
            let has_attr_default = salad_attrs.contains(SALAD_ATTR_DEFAULT);

            match (optional, primitive, has_attr_default) {
                (true, false, true) | (false, false, _) => {
                    quote!( Some(self::#value_ident::#value_variant(v)) => v, )
                }
                (true, true, true) | (false, true, _) => {
                    quote!( Some(self::#value_ident::#value_variant(v)) => *v, )
                }
                (true, false, false) => quote! {
                    Some(self::#value_ident::#value_variant(v)) => Some(v),
                    None => None,
                },
                (true, true, false) => quote! {
                    Some(self::#value_ident::#value_variant(v)) => Some(*v),
                    None => None,
                },
            }
        });

        Ok(quote! {
            #[automatically_derived]
            impl self::#ident {
                #(
                    #method_docs
                    pub fn #method_ident(&self) -> #method_return {
                        match self.0.get(#field_literal) {
                            #field_matches
                            // TODO Must be rechecked when doing the builder
                            // TODO To be bench against unreachable macro
                            _ => {
                                debug_assert!(false, "The struct field has wrong type/is None.");
                                unsafe { _std::hint::unreachable_unchecked() }
                            },
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
        })
    }
}

mod de {
    use syn::Lit;

    use super::*;
    use crate::metadata::{
        StructField, SALAD_ATTR_MAP_KEY, SALAD_ATTR_MAP_PREDICATE, SALAD_ATTR_SUBSCOPE,
    };

    fn check_field_value_iter<'a>(
        input: &'a InputStruct,
    ) -> impl Iterator<Item = TokenStream2> + 'a {
        let InputStruct {
            fields,
            value_ident,
            ..
        } = &input;

        fields.iter().filter_map(move |f| {
            let StructField {
                salad_attrs,
                ty,
                literal,
                variant_ident,
                ..
            } = f;

            let default_value = salad_attrs.get(SALAD_ATTR_DEFAULT);
            let (mandatory, ty) = {
                let subty = ty.sub_type(Some("Option"));
                (subty.is_none(), subty.unwrap_or(ty))
            };

            match (mandatory, default_value) {
                (_, Some(Lit::Str(value))) => {
                    let error_str =
                        format!("the field `{literal}` can not set to `{}`", value.value());

                    Some(quote! {
                        if !struct_map.contains_key(#literal) {
                            let key = _compact_str::CompactString::from(#literal);
                            let value = match #ty::try_from(#value) {
                                Ok(v) => self::#value_ident::#variant_ident(v),
                                Err(_) => {
                                    debug_assert!(false, #error_str);
                                    unsafe { _std::hint::unreachable_unchecked() }
                                }
                            };

                            struct_map.insert(key, value);
                        }
                    })
                }
                (_, Some(value)) => Some(quote! {
                    if !struct_map.contains_key(#literal) {
                        let key = _compact_str::CompactString::from(#literal);
                        let value = self::#value_ident::#variant_ident(#ty::from(#value));
                        struct_map.insert(key, value);
                    }
                }),
                (true, None) => {
                    let error_str = format!("the field `{literal}` is null");

                    Some(quote! {
                        if !struct_map.contains_key(#literal) {
                            return Err(_serde::de::Error::custom(#error_str));
                        }
                    })
                }
                (false, None) => None,
            }
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
        let mandatory_fields = check_field_value_iter(input);
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
                            quote!( crate::__private::de::list::MapOrSeqDeserializeSeed::new(#key, None, self.0) )
                        }
                        (Some(key), Some(pred)) => {
                            quote!( crate::__private::de::list::MapOrSeqDeserializeSeed::new(#key, Some(#pred), self.0) )
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
                    struct StructVisitor<'_sd>(&'_sd crate::__private::de::SeedData);

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
                                use crate::__private::de::IntoDeserializeSeed;

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

                                #( #mandatory_fields )*

                                struct_map
                            };

                            if is_id_declared {
                                self.0.pop_parent_id();
                            }

                            Ok(self::#ident(crate::__private::Ref::new(struct_map)))
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

        let mandatory_fields = check_field_value_iter(input);
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
                            quote!( crate::__private::de::list::MapOrSeqDeserializeSeed::new(#key, None, self.0) )
                        }
                        (Some(key), Some(pred)) => {
                            quote!( crate::__private::de::list::MapOrSeqDeserializeSeed::new(#key, Some(#pred), self.0) )
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
                    struct StructVisitor<'_sd>(&'_sd crate::__private::de::SeedData);

                    impl<'_sd, '_de> _serde::de::Visitor<'_de> for StructVisitor<'_sd> {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut _std::fmt::Formatter) -> _std::fmt::Result {
                            f.write_str(#expected_str)
                        }

                        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                        where
                            A: _serde::de::MapAccess<'_de>,
                        {
                            use crate::__private::de::IntoDeserializeSeed;

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

                            #( #mandatory_fields )*

                            Ok(self::#ident(crate::__private::Ref::new(struct_map)))
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
