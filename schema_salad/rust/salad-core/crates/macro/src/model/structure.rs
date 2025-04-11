use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Token, Type, Visibility,
};

use super::Attributes;

/// ...
pub struct InputStruct {
    pub fields: Vec<Field>,
}

/// ...
pub struct Field {
    pub attrs: Attributes,
    pub vis: Visibility,
    pub ident: Ident,
    pub ty: Type,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.parse::<Attributes>()?;
        let vis = input.parse::<Visibility>()?;
        let ident = input.parse::<Ident>()?;
        let _ = input.parse::<Token![:]>()?;
        let ty = input.parse::<Type>()?;

        Ok(Self {
            attrs,
            vis,
            ident,
            ty,
        })
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            attrs,
            vis,
            ident,
            ty,
        } = self;

        tokens.append_all(quote! {
            #attrs
            #vis #ident: #ty
        })
    }
}
