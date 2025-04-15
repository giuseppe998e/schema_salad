use proc_macro::TokenStream;

mod codegen;
mod model;
mod util;

use self::{codegen::macro_codegen, model::MacroInput};

/// TODO ...
#[proc_macro]
pub fn salad_type(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as MacroInput);
    macro_codegen(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
