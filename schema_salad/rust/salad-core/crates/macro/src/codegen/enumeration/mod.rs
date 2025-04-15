use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

mod de;

use crate::model::{attributes, enumeration::InputEnum, MacroInput};

pub fn generate(input: &MacroInput, kind: &InputEnum) -> syn::Result<TokenStream2> {
    let MacroInput {
        attrs, vis, ident, ..
    } = input;
    let variants = &kind.variants;

    let variant_ident_iter = variants.iter().map(|v| &v.ident);
    let variant_ty_iter = variants.iter().map(|v| &v.ty);

    let de_impl = de::generate(input, kind)?;

    Ok(quote! {
        #attrs
        #vis enum #ident {
            #( #variants, )*
        }

        #[doc(hidden)]
        const _: () = {
            extern crate salad_core as __core;
            use __core::__private;

            #[automatically_derived]
            impl __core::SaladType for #ident {}

            #(
                #[automatically_derived]
                impl From<#variant_ty_iter> for #ident {
                    fn from(value: #variant_ty_iter) -> Self {
                        Self::#variant_ident_iter(value)
                    }
                }
            )*

            #de_impl
        };
    })
}

pub fn generate_unit(input: &MacroInput, kind: &InputEnum) -> syn::Result<TokenStream2> {
    let MacroInput {
        attrs, vis, ident, ..
    } = input;
    let variants = &kind.variants;

    let variant_idents = variants.iter().map(|v| &v.ident).collect::<Vec<_>>();
    let variant_lits = kind
        .variants
        .iter()
        .map(|v| match v.attrs.get_str(attributes::AS_STR) {
            Some(l) => Ok(l),
            None => Err(syn::Error::new_spanned(
                v,
                format_args!(
                    "a unit-variant must have the salad attribute `{}`",
                    attributes::AS_STR
                ),
            )),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let de_impl = de::generate_unit(input, &variant_idents, &variant_lits);

    Ok(quote! {
        #attrs
        #vis enum #ident {
            #( #variants, )*
        }

        #[doc(hidden)]
        const _: () = {
            extern crate salad_core as __core;
            use __core::__private;

            #[automatically_derived]
            impl __core::SaladType for #ident {}

            #[automatically_derived]
            impl core::fmt::Display for #ident {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str(match self {
                        #( Self::#variant_idents => #variant_lits, )*
                    })
                }
            }

            #de_impl
        };
    })
}
