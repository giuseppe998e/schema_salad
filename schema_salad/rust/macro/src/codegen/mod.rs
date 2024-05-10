mod enums;
mod structs;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::input::{MacroInput, Metadata, UnitInput};

// Code generation function
pub(super) fn generate(input: MacroInput) -> syn::Result<TokenStream2> {
    let codegen: &dyn CodeGen = match &input {
        MacroInput::Struct(s) => s,
        MacroInput::Enum(e) => e,
        MacroInput::Unit(u) => u,
    };

    let ident = codegen.ident();
    let type_def = codegen.define_type()?;

    let methods_impl = codegen.impl_methods()?;
    let std_traits_impl = codegen.impl_std_traits()?;

    let serialize_impl = codegen.impl_serialize()?;
    let deserialize_seed_impl = codegen.impl_deserialize_seed()?;

    let deserialize_impl = codegen.impl_deserialize()?;

    Ok(quote! {
        #type_def

        #[doc(hidden)]
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            #[automatically_derived]
            impl crate::core::SaladType for self::#ident {}

            #methods_impl
            #std_traits_impl

            #serialize_impl
            #deserialize_seed_impl

            #deserialize_impl
        };
    })
}

// Code generation trait
pub(crate) trait CodeGen: Metadata {
    fn define_type(&self) -> syn::Result<TokenStream2>;

    #[inline]
    fn impl_methods(&self) -> syn::Result<Option<TokenStream2>> {
        Ok(None)
    }

    #[inline]
    fn impl_std_traits(&self) -> syn::Result<Option<TokenStream2>> {
        Ok(None)
    }

    fn impl_serialize(&self) -> syn::Result<TokenStream2>;
    fn impl_deserialize_seed(&self) -> syn::Result<TokenStream2>;

    #[inline]
    fn impl_deserialize(&self) -> syn::Result<Option<TokenStream2>> {
        self.__impl_deserialize()
    }

    #[doc(hidden)]
    fn __impl_deserialize(&self) -> syn::Result<Option<TokenStream2>> {
        if self.salad_attrs().document_root() {
            let ident = self.ident();

            Ok(Some(quote! {
                #[automatically_derived]
                impl<'_de> ::serde::de::Deserialize<'_de> for self::#ident {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where D: ::serde::de::Deserializer<'_de>,
                    {
                        let data = crate::__private::de::SeedData::new();
                        let dseed =
                            <Self as crate::__private::de::IntoDeserializeSeed<'_de, '_>>::into_dseed(&data);

                        #[cfg(feature = "dsl")]
                        let deserializer = crate::__private::dsl::Preprocessor::new(deserializer);

                        ::serde::de::DeserializeSeed::deserialize(dseed, deserializer)
                    }
                }
            }))
        } else {
            Ok(None)
        }
    }
}

// Unit input type
impl CodeGen for UnitInput {
    fn define_type(&self) -> syn::Result<TokenStream2> {
        let attrs = &self.attrs;
        let vis = &self.vis;
        let ident = self.ident();

        Ok(quote! {
            #(#attrs)*
            #vis struct #ident;
        })
    }

    fn impl_std_traits(&self) -> syn::Result<Option<TokenStream2>> {
        let ident = self.ident();
        let literal = &self.literal;

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
                        #literal => Ok(Self),
                        _ => Err(())
                    }
                }
            }

            #[automatically_derived]
            impl ::std::fmt::Display for self::#ident {
                #[inline]
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    f.write_str(#literal)
                }
            }
        }))
    }

    fn impl_serialize(&self) -> syn::Result<TokenStream2> {
        let ident = self.ident();
        let literal = &self.literal;

        Ok(quote! {
            #[automatically_derived]
            impl ::serde::ser::Serialize for self::#ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::ser::Serializer,
                {
                    serializer.collect_str(#literal)
                }
            }
        })
    }

    fn impl_deserialize_seed(&self) -> syn::Result<TokenStream2> {
        let ident = self.ident();

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

    fn impl_deserialize(&self) -> syn::Result<Option<TokenStream2>> {
        let ident = self.ident();
        let literal = &self.literal;

        let expect_str = format!("a string of value \"{}\"", literal.value());

        Ok(Some(quote! {
            #[automatically_derived]
            impl<'_de> ::serde::de::Deserialize<'_de> for self::#ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: ::serde::de::Deserializer<'_de>,
                {
                    struct UnitVisitor;

                    impl<'_de> ::serde::de::Visitor<'_de> for UnitVisitor {
                        type Value = self::#ident;

                        fn expecting(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            f.write_str(#expect_str)
                        }

                        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                        where E: ::serde::de::Error,
                        {
                            match v {
                                #literal => Ok(self::#ident),
                                _ => Err(::serde::de::Error::invalid_value(
                                    ::serde::de::Unexpected::Str(v),
                                    &#literal,
                                )),
                            }
                        }
                    }

                    deserializer.deserialize_str(UnitVisitor)
                }
            }
        }))
    }
}
