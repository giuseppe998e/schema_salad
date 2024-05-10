use std::collections::HashSet;

use fxhash::FxBuildHasher;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

use super::CodeGen;
use crate::{
    ext::TypeExt,
    input::{attr_keys, Metadata, StructInput},
};

impl CodeGen for StructInput {
    fn define_type(&self) -> syn::Result<TokenStream2> {
        let attrs = &self.attrs;
        let vis = &self.vis;
        let ident = self.ident();
        let seed_ident = self.seed_ident();
        let value_ident = &self.value_ident;

        let mut value_variants_seen =
            HashSet::with_capacity_and_hasher(self.fields.len(), FxBuildHasher::default());
        let value_variant_iter = self.fields.iter().filter_map(|f| {
            let variant_ident = &f.variant_ident;
            let ty = f.ty.sub_type(Some("Option")).unwrap_or(&f.ty);
            (value_variants_seen.insert(variant_ident) && variant_ident != "Any")
                .then(|| quote!( #variant_ident(#ty) ))
        });

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
            #[derive(Clone)]
            enum #value_ident {
                #( #value_variant_iter, )*
                Any(crate::core::Any),
            }

            #[doc(hidden)]
            pub(crate) struct #seed_ident<'_sd>(&'_sd crate::__private::de::SeedData);
        })
    }

    fn impl_methods(&self) -> syn::Result<Option<TokenStream2>> {
        let ident = self.ident();
        let value_ident = &self.value_ident;

        let getter_method_iter = self.fields.iter().map(|f| {
            let field_ident = &f.ident;
            let field_literal = &f.literal;
            let has_attr_default = f.salad_attrs.contains(attr_keys::DEFAULT);

            let method_docs = f.attrs.iter().filter(|a| a.path().is_ident("doc")).fold(
                TokenStream2::new(),
                |mut ts, a| {
                    a.to_tokens(&mut ts);
                    ts
                },
            );

            let method_return = if has_attr_default {
                let ty = f.ty.sub_type(Some("Option")).unwrap_or(&f.ty).clone();
                ty.into_typeref()
            } else {
                f.ty.clone().into_typeref()
            };

            let variant_match = {
                let variant_ident = &f.variant_ident;
                let (ty_optional, ty_primitive) = {
                    let opt_ty = f.ty.sub_type(Some("Option"));
                    let ty = opt_ty.unwrap_or(&f.ty);
                    (opt_ty.is_some(), ty.is_salad_primitive())
                };

                match (ty_optional, ty_primitive, has_attr_default) {
                    (true, false, true) | (false, false, _) => {
                        quote!( Some(self::#value_ident::#variant_ident(v)) => v, )
                    }
                    (true, true, true) | (false, true, _) => {
                        quote!( Some(self::#value_ident::#variant_ident(v)) => *v, )
                    }
                    (true, false, false) => quote! {
                        Some(self::#value_ident::#variant_ident(v)) => Some(v),
                        None => None,
                    },
                    (true, true, false) => quote! {
                        Some(self::#value_ident::#variant_ident(v)) => Some(*v),
                        None => None,
                    },
                }
            };

            quote! {
                #method_docs
                pub fn #field_ident(&self) -> #method_return {
                    match self.0.get(#field_literal) {
                        #variant_match
                        _ => {
                            debug_assert!(false, "The struct field has wrong type/is None.");
                            unsafe { ::std::hint::unreachable_unchecked() }
                        },
                    }
                }
            }
        });

        Ok(Some(quote! {
            #[automatically_derived]
            impl self::#ident {
                #( #getter_method_iter )*

                // TODO Method documentation
                pub fn get_extension(&self, key: &str) -> Option<&crate::core::Any> {
                    match self.0.get(key) {
                        Some(self::#value_ident::Any(v)) => Some(v),
                        _ => None,
                    }
                }
            }
        }))
    }

    fn impl_std_traits(&self) -> syn::Result<Option<TokenStream2>> {
        let value_ident = &self.value_ident;

        let mut value_variants_seen =
            HashSet::with_capacity_and_hasher(self.fields.len(), FxBuildHasher::default());
        let value_variant_iter = self.fields.iter().filter_map(|f| {
            let variant_ident = &f.variant_ident;
            (value_variants_seen.insert(variant_ident) && variant_ident != "Any")
                .then_some(variant_ident)
        });

        Ok(Some(quote! {
            #[automatically_derived]
            impl ::std::fmt::Debug for self::#value_ident {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    match self {
                        #( Self::#value_variant_iter(v) => ::std::fmt::Debug::fmt(v, f), )*
                        Self::Any(v) => ::std::fmt::Debug::fmt(v, f),
                    }
                }
            }
        }))
    }

    fn impl_serialize(&self) -> syn::Result<TokenStream2> {
        let ident = self.ident();
        let value_ident = &self.value_ident;

        let mut value_variants_seen =
            HashSet::with_capacity_and_hasher(self.fields.len(), FxBuildHasher::default());
        let value_variant_iter = self.fields.iter().filter_map(|f| {
            let variant_ident = &f.variant_ident;
            (value_variants_seen.insert(variant_ident) && variant_ident != "Any")
                .then_some(variant_ident)
        });

        Ok(quote! {
            #[automatically_derived]
            impl ::serde::ser::Serialize for self::#ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::ser::Serializer,
                {
                    self.0.serialize(serializer)
                }
            }

            #[automatically_derived]
            impl ::serde::ser::Serialize for self::#value_ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::ser::Serializer,
                {
                    match self {
                        #( Self::#value_variant_iter(v) => v.serialize(serializer), )*
                        Self::Any(v) => v.serialize(serializer),
                    }
                }
            }
        })
    }

    fn impl_deserialize_seed(&self) -> syn::Result<TokenStream2> {
        let ident = self.ident();
        let seed_ident = self.seed_ident();
        let value_ident = &self.value_ident;

        let expect_str = format!("a valid {ident} object");
        let field_count = self.fields.len();

        let (fields, field_id) = {
            let mut field_id = None;
            (
                self.fields
                    .iter()
                    .filter(|f| {
                        if f.salad_attrs.identifier() {
                            field_id = Some(*f);
                            false
                        } else {
                            true
                        }
                    })
                    .collect::<Vec<_>>(),
                field_id,
            )
        };

        //
        let pre_deserialize = field_id.map(|f| {
            let f_literal = &f.literal;
            let f_variant_ident = &f.variant_ident;

            quote! {
                let mut is_id_set = false;
                let mut map = {
                    let mut fields_cache = Vec::with_capacity(#field_count);

                    while let Some(key) = map.next_key::<::serde::__private::de::Content<'_de>>()? {
                        match key.as_str() {
                            Some(#f_literal) => {
                                let value = ::serde::de::MapAccess::next_value::<::compact_str::CompactString>(&mut map)?;
                                match self.0.generate_id(value).map(crate::core::StrValue::from) {
                                    Ok(value) => {
                                        let key = ::compact_str::CompactString::from(#f_literal);
                                        fields.insert(key, self::#value_ident::#f_variant_ident(value));
                                        is_id_set = true;
                                    }
                                    Err(e) => {
                                        return Err(::serde::de::Error::custom(e))
                                    }
                                }
                            },
                            _ => {
                                let value = ::serde::de::MapAccess::next_value::<::serde::__private::de::Content<'_de>>(&mut map)?;
                                fields_cache.push((key, value));
                            },
                        }
                    }

                    crate::__private::de::VecMapAccess::from(fields_cache)
                };
            }
        });

        let post_deserialize = field_id.map(|_| {
            quote! {
                if is_id_set { self.0.pop_parent_id(); }
            }
        });

        //
        let field_match_iter = fields.iter().map(|f| {
            let f_literal = &f.literal;
            let f_variant_ident = &f.variant_ident;

            let dseed_expr = {
                let f_ty = f.ty.sub_type(Some("Option")).unwrap_or(&f.ty);
                let f_map_key = f.salad_attrs.map_key();
                let f_map_predicate = f.salad_attrs.map_predicate();

                match (f_map_key, f_map_predicate) {
                    (Some(key), Some(pred)) => quote! {
                        crate::__private::de::MapOrSeqDeserializeSeed::new(#key, Some(#pred), self.0)
                    },
                    (Some(key), None) => quote! {
                        crate::__private::de::MapOrSeqDeserializeSeed::new(#key, None, self.0)
                    },
                    (None, _) => quote! {
                        <#f_ty as crate::__private::de::IntoDeserializeSeed>::into_dseed(self.0)
                    },
                }
            };

            let (subscope_push, subscope_pop) = match f.salad_attrs.identifier_subscope() {
                Some(subscope) => (
                    Some(quote!( self.0.push_subscope(#subscope); )),
                    Some(quote!( self.0.pop_parent_id(); )),
                ),
                None => (None, None),
            };

            quote! {
                #f_literal => {
                    #subscope_push
                    let value = ::serde::de::MapAccess::next_value_seed(&mut map, #dseed_expr)?;
                    #subscope_pop
                    self::#value_ident::#f_variant_ident(value)
                }
            }
        });

        //
        let mandatory_field_iter = fields.iter().filter_map(|f| {
            let f_literal = &f.literal;
            let f_variant_ident = &f.variant_ident;
            let f_default_value = f.salad_attrs.default_value();
            let (is_mandatory, f_ty) = {
                let subty = f.ty.sub_type(Some("Option"));
                (subty.is_none(), subty.unwrap_or(&f.ty))
            };

            let if_body = match (f_default_value, is_mandatory) {
                (Some(syn::Lit::Str(val)), _) => {
                    let error_str = format!(
                        "the field `{f_literal}` can not be set to `{}`",
                        val.value()
                    );

                    quote! {
                        let key = ::compact_str::CompactString::from(#f_literal);
                        let value = <#f_ty as ::std::str::FromStr>::from_str(#val)
                            .map_err(|_| ::serde::de::Error::custom(#error_str))
                            .map(self::#value_ident::#f_variant_ident)?;
                        fields.insert(key, value);
                    }
                }
                (Some(val), _) => quote! {
                    let key = ::compact_str::CompactString::from(#f_literal);
                    let value = self::#value_ident::#f_variant_ident(#f_ty::from(#val));
                    fields.insert(key, value);
                },
                (None, true) => {
                    let error_str = format!("the field `{f_literal}` is null");
                    quote! { return Err(::serde::de::Error::custom(#error_str)); }
                }
                (None, false) => return None,
            };

            Some(quote! {
                if !fields.contains_key(#f_literal) {
                    #if_body
                }
            })
        });

        Ok(quote! {
            #[automatically_derived]
            impl<'_de, '_sd> crate::__private::de::IntoDeserializeSeed<'_de, '_sd> for self::#ident {
                type Value = self::#seed_ident<'_sd>;

                #[inline]
                fn into_dseed(data: &'_sd crate::__private::de::SeedData) -> Self::Value {
                    self::#seed_ident(data)
                }
            }

            #[automatically_derived]
            impl<'_de, '_sd> ::serde::de::DeserializeSeed<'_de> for self::#seed_ident<'_sd> {
                type Value = self::#ident;

                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where D: ::serde::de::Deserializer<'_de>,
                {
                    impl<'_de, '_sd> ::serde::de::Visitor<'_de> for self::#seed_ident<'_sd> {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            f.write_str(#expect_str)
                        }

                        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                        where A: ::serde::de::MapAccess<'_de>,
                        {
                            let mut fields = ::std::collections::HashMap::with_capacity_and_hasher(
                                #field_count, ::fxhash::FxBuildHasher::default()
                            );

                            #pre_deserialize

                            while let Some(key) = ::serde::de::MapAccess::next_key::<::compact_str::CompactString>(&mut map)? {
                                if fields.contains_key(&key) {
                                    return Err(::serde::de::Error::custom(format_args!(
                                        "duplicate field `{}`",
                                        key
                                    )));
                                }

                                let value = match key.as_str() {
                                    #( #field_match_iter, )*
                                    _ => {
                                        let value = ::serde::de::MapAccess::next_value_seed(
                                            &mut map,
                                            <crate::core::Any as crate::__private::de::IntoDeserializeSeed>::into_dseed(self.0),
                                        )?;
                                        self::#value_ident::Any(value)
                                    }
                                };

                                fields.insert(key, value);
                            }

                            #post_deserialize
                            #( #mandatory_field_iter )*

                            Ok(self::#ident(crate::__private::Ref::new(fields)))
                        }
                    }

                    deserializer.deserialize_map(self)
                }
            }
        })
    }
}
