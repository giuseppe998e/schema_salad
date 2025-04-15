use proc_macro2::TokenStream as TokenStream2;

mod enumeration;
mod structure;

use crate::model::{InputKind, MacroInput};

pub fn macro_codegen(input: MacroInput) -> syn::Result<TokenStream2> {
    match &input.kind {
        InputKind::Enum(kind) => enumeration::generate(&input, kind),
        InputKind::Struct(kind) => structure::generate(&input, kind),

        InputKind::UnitEnum(kind) => enumeration::generate_unit(&input, kind),
        InputKind::UnitStruct => structure::generate_unit(&input),
    }
}
