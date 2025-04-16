use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    token, Attribute, Ident, Type,
};

use super::SaladAttrs;

/// TODO ...
pub struct InputEnum {
    pub variants: Vec<Variant>,
}

/// TODO ...
pub struct Variant {
    pub salad: SaladAttrs,
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub ty: Option<Type>,
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (salad, attrs) = {
            let attrs = input.call(Attribute::parse_outer)?;
            SaladAttrs::parse(attrs)?
        };

        let ident = input.parse::<Ident>()?;

        let lookahead = input.lookahead1();
        let ty = if lookahead.peek(token::Paren) {
            let content;
            let _ = syn::parenthesized!(content in input);
            content.parse::<Type>().map(Some)?
        } else {
            None
        };

        Ok(Self {
            salad,
            attrs,
            ident,
            ty,
        })
    }
}

impl ToTokens for Variant {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            attrs, ident, ty, ..
        } = self;
        let ty = ty.iter();

        tokens.append_all(quote! {
            #( #attrs )*
            #ident #((#ty))* // used as optional
        });
    }
}
