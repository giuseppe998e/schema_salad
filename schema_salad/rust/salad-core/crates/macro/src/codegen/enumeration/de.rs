use compact_str::format_compact;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident, LitStr};

use crate::model::{enumeration::InputEnum, MacroInput};

/// TODO ...
pub fn generate(input: &MacroInput, kind: &InputEnum) -> syn::Result<TokenStream2> {
    let MacroInput { ident, .. } = input;
    let seed_ident = format_ident!("__{ident}Seed");
    let visitor_ident = format_ident!("{ident}Visitor");

    Ok(quote! {
        #[doc(hidden)]
        pub struct #seed_ident<'__sd>(&'__sd __private::de::SeedData);

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
pub fn generate_unit(
    input: &MacroInput,
    variant_idents: &[&Ident],
    variant_lits: &[&LitStr],
) -> TokenStream2 {
    let MacroInput { ident, .. } = input;
    let visitor_ident = format_ident!("{ident}Visitor");

    let expecting_str = unit_expecting_str(variant_lits);

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
                            #( #variant_lits => Ok(#ident::#variant_idents), )*
                            v => Err(__private::de::Error::invalid_value(__private::de::Unexpected::Str(v), &self)),
                        }
                    }
                }

                deserializer.deserialize_str(#visitor_ident)
            }
        }
    }
}

fn unit_expecting_str(variant_lits: &[&LitStr]) -> String {
    let Some(lit) = variant_lits.first() else {
        unreachable!();
    };

    let mut result = format!("a string with value `{}`", lit.value());

    if let Some((last_lit, lits)) = variant_lits[1..].split_last() {
        result.extend(lits.iter().map(|l| format_compact!(", `{}`", l.value())));
        result.push_str(&format_compact!(" or `{}`", last_lit.value()));
    }

    result
}
