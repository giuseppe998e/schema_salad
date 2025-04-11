use proc_macro::TokenStream;

mod model;
mod util;

use self::model::MacroInput;

/// TODO ...
#[proc_macro]
pub fn salad_type(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as MacroInput);
    panic!("parsing works!")
}
