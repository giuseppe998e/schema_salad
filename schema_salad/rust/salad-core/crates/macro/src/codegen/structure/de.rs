use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::LitStr;

use crate::model::{attributes, structure::InputStruct, MacroInput};

/// TODO ...
pub fn generate(input: &MacroInput, kind: &InputStruct) -> syn::Result<TokenStream2> {
    let MacroInput { ident, .. } = input;
    let seed_ident = format_ident!("__{ident}Seed");
    let visitor_ident = format_ident!("{ident}Visitor");

    Ok(quote! {
        #[doc(hidden)]
        pub struct #seed_ident<'__sd>(&'__sd __private::de::SeedData)

        #[automatically_derived]
        impl<'__de, '__sd> __private::de::IntoDeserializeSeed<'__de, '__sd> for #ident {
            type DeserializeSeed = #seed_ident<'__sd>;

            fn deserialize_seed(data: &'__sd __private::de::SeedData) -> Self::DeserializeSeed {
                #seed_ident(data)
            }
        }

        #[automatically_derived]
        impl<'__de, '__sd> __private::de::DeserializeSeed<'__de> for #seed_ident<'__sd> {
            type Value = #ident;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: __private::de::Deserializer<'__de>,
            {
                struct #visitor_ident<'__sd>(&'__sd __private::de::SeedData);

                todo!()
            }
        }
    })
}

/// TODO ...
pub fn generate_unit(input: &MacroInput, struct_lit: &LitStr) -> TokenStream2 {
    let MacroInput { ident, .. } = input;
    let visitor_ident = format_ident!("{ident}Visitor");

    let expecting_str = format!("a string with value `{}`", struct_lit.value());

    quote! {
        #[automatically_derived]
        impl<'__de, '__sd> __private::de::IntoDeserializeSeed<'__de, '__sd> for #ident {
            type DeserializeSeed = core::marker::PhantomData<#ident>;

            fn deserialize_seed(_: &'__sd __private::de::SeedData) -> Self::DeserializeSeed {
                core::marker::PhantomData
            }
        }

        #[automatically_derived]
        impl<'__de> __private::de::Deserialize<'__de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: __private::de::Deserializer<'__de>,
            {
                struct #visitor_ident;

                impl<'__de> __private::de::Visitor<'__de> for #visitor_ident {
                    type Value = #ident;

                    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                        f.write_str(#expecting_str)
                    }

                    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                    where
                        E: __private::de::Error,
                    {
                        match s {
                            #struct_lit => Ok(#ident),
                            v => Err(__private::de::Error::invalid_value(__private::de::Unexpected::Str(v), &self)),
                        }
                    }
                }

                deserializer.deserialize_str(#visitor_ident)
            }
        }
    }
}
