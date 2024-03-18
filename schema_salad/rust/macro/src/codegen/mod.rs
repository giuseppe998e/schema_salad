mod enums;
mod structs;
mod units;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use self::{enums::generate_enum, structs::generate_struct, units::generate_unit};
use crate::metadata::{MacroAttributes, MacroInput, SALAD_ATTR_ROOT};

pub(super) fn generate_schema_type(input: MacroInput) -> syn::Result<TokenStream2> {
    match input {
        MacroInput::Struct(input) => generate_struct(input),
        MacroInput::Enum(input) => generate_enum(input),
        MacroInput::Unit(input) => generate_unit(input),
    }
}

fn generate_root_de_impl(
    ident: &Ident,
    seed_ident: &Ident,
    salad_attrs: &MacroAttributes,
) -> Option<TokenStream2> {
    if salad_attrs.contains_and_is_true(SALAD_ATTR_ROOT) {
        Some(quote! {
            #[automatically_derived]
            impl<'_de> _serde::de::Deserialize<'_de> for #ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: _serde::de::Deserializer<'_de>
                {
                    let data = crate::util::de::SeedData::new();
                    let seed = #seed_ident(&data);
                    _serde::de::DeserializeSeed::deserialize(seed, deserializer)
                }
            }
        })
    } else {
        None
    }
}
