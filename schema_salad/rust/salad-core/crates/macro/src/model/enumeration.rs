use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    token, Ident, Type,
};

use super::Attributes;

/// ...
pub struct InputEnum {
    pub variants: Vec<Variant>,
}

/// ...
pub struct Variant {
    pub attrs: Attributes,
    pub ident: Ident,
    pub ty: Option<Type>,
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.parse::<Attributes>()?;
        let ident = input.parse::<Ident>()?;

        let lookahead = input.lookahead1();
        let ty = if lookahead.peek(token::Paren) {
            let content;
            let _ = syn::parenthesized!(content in input);
            content.parse::<Type>().map(Some)?
        } else {
            None
        };

        Ok(Self { attrs, ident, ty })
    }
}

impl ToTokens for Variant {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { attrs, ident, ty } = self;
        let ty = ty.iter();

        tokens.append_all(quote! {
            #attrs
            #ident #((#ty))*
        });
    }
}
