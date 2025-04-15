use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

mod de;

use crate::model::{attributes, structure::InputStruct, MacroInput};

pub fn generate(input: &MacroInput, kind: &InputStruct) -> syn::Result<TokenStream2> {
    let MacroInput {
        attrs, vis, ident, ..
    } = input;

    let fields = &kind.fields;
    let de = de::generate(input, kind)?;

    Ok(quote! {
        #attrs
        #vis struct #ident {
            #( #fields, )*
        }

        #[doc(hidden)]
        const _: () = {
            extern crate salad_core as __core;
            use __core::__private;

            #[automatically_derived]
            impl __core::SaladType for #ident {}

            #de
        };
    })
}

pub fn generate_unit(input: &MacroInput) -> syn::Result<TokenStream2> {
    let MacroInput {
        attrs, vis, ident, ..
    } = input;

    let struct_lit = match attrs.get_str(attributes::AS_STR) {
        Some(l) => l,
        None => {
            return Err(syn::Error::new_spanned(
                ident,
                format_args!(
                    "a unit-struct must have the salad attribute `{}`",
                    attributes::AS_STR
                ),
            ))
        }
    };

    let de_impl = de::generate_unit(input, struct_lit);

    Ok(quote! {
        #attrs
        #vis struct #ident;

        #[doc(hidden)]
        const _: () = {
            extern crate salad_core as __core;
            use __core::__private;

            #[automatically_derived]
            impl __core::SaladType for #ident {}

            #[automatically_derived]
            impl core::fmt::Display for #ident {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str(#struct_lit)
                }
            }

            #de_impl
        };
    })
}
