use proc_macro::TokenStream;
use syn::parse_macro_input;

mod codegen;
mod metadata;
mod util;

#[proc_macro]
pub fn define_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as metadata::MacroInput);
    codegen::generate_schema_type(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
