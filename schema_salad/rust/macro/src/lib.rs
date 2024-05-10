mod codegen;
mod ext;
mod input;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn define_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as input::MacroInput);
    codegen::generate(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
