use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{punctuated::Punctuated, Token};

use super::CodeGen;
use crate::input::{EnumInput, Metadata, Variants};

impl CodeGen for EnumInput {
    fn define_type(&self) -> syn::Result<TokenStream2> {
        let attrs = &self.attrs;
        let vis = &self.vis;
        let ident = self.ident();
        let variants = &self.variants;

        let seed_struct = match variants {
            Variants::Tuple(_) => {
                let seed_ident = self.seed_ident();
                Some(quote! {
                    #[doc(hidden)]
                    pub(crate) struct #seed_ident<'_sd>(&'_sd crate::__private::de::SeedData);
                })
            }
            Variants::Unit(_) => None,
        };

        Ok(quote! {
            #(#attrs)*
            #vis enum #ident {
                #variants
            }

            #seed_struct
        })
    }

    fn impl_std_traits(&self) -> syn::Result<Option<TokenStream2>> {
        match &self.variants {
            Variants::Tuple(variants) => tuple::impl_std_traits(self, variants),
            Variants::Unit(variants) => unit::impl_std_traits(self, variants),
        }
    }

    fn impl_serialize(&self) -> syn::Result<TokenStream2> {
        match &self.variants {
            Variants::Tuple(variants) => tuple::impl_serialize(self, variants),
            Variants::Unit(_) => unit::impl_serialize(self),
        }
    }

    fn impl_deserialize_seed(&self) -> syn::Result<TokenStream2> {
        match &self.variants {
            Variants::Tuple(variants) => tuple::impl_deserialize_seed(self, variants),
            Variants::Unit(_) => unit::impl_deserialize_seed(self),
        }
    }

    fn impl_deserialize(&self) -> syn::Result<Option<TokenStream2>> {
        match &self.variants {
            Variants::Tuple(_) => self.__impl_deserialize(),
            Variants::Unit(variants) => unit::impl_deserialize(self, variants),
        }
    }
}

mod tuple {
    use super::*;
    use crate::{ext::TypeExt, input::TupleVariant};

    pub(super) fn impl_std_traits(
        this: &EnumInput,
        variants: &Punctuated<TupleVariant, Token![,]>,
    ) -> syn::Result<Option<TokenStream2>> {
        let ident = this.ident();
        let tokenstream = variants
            .iter()
            .map(move |v| {
                let variant_ident = &v.ident;
                let variant_ty = &v.field.ty;

                let custom_from_impls = {
                    let variant_ty_discr = variant_ty.to_variant_ident();
                    if variant_ty_discr == "StrValue" {
                        Some(quote! {
                            #[automatically_derived]
                            impl ::std::convert::From<&str> for self::#ident {
                                fn from(value: &str) -> Self {
                                    Self::#variant_ident(
                                        ::std::convert::Into::into(value)
                                    )
                                }
                            }

                            #[automatically_derived]
                            impl ::std::convert::From<String> for self::#ident {
                                fn from(value: String) -> Self {
                                    Self::#variant_ident(
                                        ::std::convert::Into::into(value)
                                    )
                                }
                            }
                        })
                    } else if variant_ty_discr == "ListStrValue" {
                        Some(quote! {
                            #[automatically_derived]
                            impl ::std::convert::From<&[&str]> for self::#ident {
                                fn from(value: &[&str]) -> Self {
                                    let box_slice = value
                                            .iter()
                                            .map(|s| ::std::convert::Into::into(*s))
                                            .collect::<crate::core::List<_>>();
                                    Self::#variant_ident(box_slice)
                                }
                            }

                            #[automatically_derived]
                            impl ::std::convert::From<&[String]> for self::#ident {
                                fn from(value: &[String]) -> Self {
                                    let box_slice = value
                                            .iter()
                                            .map(::std::convert::Into::into)
                                            .collect::<crate::core::List<_>>();
                                    Self::#variant_ident(box_slice)
                                }
                            }

                            #[automatically_derived]
                            impl ::std::convert::From<Vec<String>> for self::#ident {
                                fn from(value: Vec<String>) -> Self {
                                    let box_slice = value
                                            .into_iter()
                                            .map(::std::convert::Into::into)
                                            .collect::<crate::core::List<_>>();
                                    Self::#variant_ident(box_slice)
                                }
                            }
                        })
                    } else {
                        None
                    }
                };

                quote! {
                    #[automatically_derived]
                    impl ::std::convert::From<#variant_ty> for self::#ident {
                        fn from(value: #variant_ty) -> Self {
                            Self::#variant_ident(value)
                        }
                    }

                    #custom_from_impls
                }
            })
            .collect();

        Ok(Some(tokenstream))
    }

    pub(super) fn impl_serialize(
        this: &EnumInput,
        variants: &Punctuated<TupleVariant, Token![,]>,
    ) -> syn::Result<TokenStream2> {
        let ident = this.ident();
        let variant_ident_iter = variants.iter().map(|v| &v.ident);

        Ok(quote! {
            #[automatically_derived]
            impl ::serde::ser::Serialize for self::#ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::ser::Serializer,
                {
                    match self {
                        #( Self::#variant_ident_iter(v) => v.serialize(serializer) ),*
                    }
                }
            }
        })
    }

    pub(super) fn impl_deserialize_seed(
        this: &EnumInput,
        variants: &Punctuated<TupleVariant, Token![,]>,
    ) -> syn::Result<TokenStream2> {
        let ident = this.ident();
        let seed_ident = this.seed_ident();

        let err_string = format!("data did not match any variant of enum {}", ident);

        let variant_ident_iter = variants.iter().map(|v| &v.ident);
        let variant_ty_iter = variants.iter().map(|v| &v.field.ty);

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
                    let content =
                        <::serde::__private::de::Content<'_de> as ::serde::Deserialize>::deserialize(
                            deserializer,
                        )?;
                    let content_deserializer =
                        ::serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content);

                    #({
                        let data = self.0.clone();
                        let dseed =
                            <#variant_ty_iter as crate::__private::de::IntoDeserializeSeed>::into_dseed(
                                &data
                            );

                        if let Ok(v) =
                            ::serde::de::DeserializeSeed::deserialize(dseed, content_deserializer)
                        {
                            return match self.0.extend(data) {
                                Ok(_) => Ok(self::#ident::#variant_ident_iter(v)),
                                Err(e) => Err(::serde::de::Error::custom(e)),
                            };
                        }
                    })*

                    Err(::serde::de::Error::custom(#err_string))
                }
            }
        })
    }
}

mod unit {
    use super::*;
    use crate::input::UnitVariant;

    pub(super) fn impl_std_traits(
        this: &EnumInput,
        variants: &Punctuated<UnitVariant, Token![,]>,
    ) -> syn::Result<Option<TokenStream2>> {
        let ident = this.ident();

        let variant_ident_iter1 = variants.iter().map(|v| &v.ident);
        let variant_ident_iter2 = variant_ident_iter1.clone();

        let variant_value_iter1 = variants.iter().map(|v| &v.literal);
        let variant_value_iter2 = variant_value_iter1.clone();

        Ok(Some(quote! {
            #[automatically_derived]
            impl ::std::convert::TryFrom<&str> for self::#ident {
                type Error = ();

                #[inline]
                fn try_from(value: &str) -> Result<Self, Self::Error> {
                    <self::#ident as ::std::str::FromStr>::from_str(value)
                }
            }

            #[automatically_derived]
            impl ::std::str::FromStr for self::#ident {
                type Err = ();

                fn from_str(input: &str) -> Result<Self, Self::Err> {
                    match input {
                        #( #variant_value_iter1 => Ok(self::#ident::#variant_ident_iter1), )*
                        _ => Err(()),
                    }
                }
            }

            #[automatically_derived]
            impl ::std::fmt::Display for self::#ident {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    match self {
                        #( Self::#variant_ident_iter2 => f.write_str(#variant_value_iter2) ),*
                    }
                }
            }
        }))
    }

    pub(super) fn impl_serialize(this: &EnumInput) -> syn::Result<TokenStream2> {
        let ident = this.ident();

        Ok(quote! {
            #[automatically_derived]
            impl ::serde::ser::Serialize for self::#ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::ser::Serializer,
                {
                    serializer.collect_str(self)
                }
            }
        })
    }

    pub(super) fn impl_deserialize_seed(this: &EnumInput) -> syn::Result<TokenStream2> {
        let ident = this.ident();

        Ok(quote! {
            #[automatically_derived]
            impl<'_de, '_sd> crate::__private::de::IntoDeserializeSeed<'_de, '_sd> for self::#ident {
                type Value = ::std::marker::PhantomData<Self>;

                #[inline]
                fn into_dseed(_: &'_sd crate::__private::de::SeedData) -> Self::Value {
                    ::std::marker::PhantomData
                }
            }
        })
    }

    pub(super) fn impl_deserialize(
        this: &EnumInput,
        variants: &Punctuated<UnitVariant, Token![,]>,
    ) -> syn::Result<Option<TokenStream2>> {
        let ident = this.ident();

        let values_str = variants
            .iter()
            .map(|v| format!("\"{}\"", v.literal.value()))
            .collect::<Vec<_>>()
            .join(", ");

        let expected_str = {
            let mut expected_str = "a string with one of the following values: ".to_owned();
            expected_str.push_str(&values_str);
            expected_str.push('.');
            expected_str
        };

        Ok(Some(quote! {
            #[automatically_derived]
            impl<'_de> ::serde::Deserialize<'_de> for self::#ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'_de>,
                {
                    struct UnitEnumVisitor;

                    impl<'_de> ::serde::de::Visitor<'_de> for UnitEnumVisitor {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            f.write_str(#expected_str)
                        }

                        fn visit_str<E: ::serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                            <self::#ident as ::std::str::FromStr>::from_str(v)
                                .map_err(|_| ::serde::de::Error::invalid_value(
                                    ::serde::de::Unexpected::Str(v),
                                    &#values_str,
                                ))
                        }
                    }

                    deserializer.deserialize_str(UnitEnumVisitor)
                }
            }
        }))
    }
}
