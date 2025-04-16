use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Ident, Token, Type, Visibility,
};

use super::SaladAttrs;

/// TODO ...
pub struct InputStruct {
    pub fields: Vec<Field>,
}

/// TODO ...
pub struct Field {
    pub salad: SaladAttrs,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub ty: Type,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (salad, attrs) = {
            let attrs = input.call(Attribute::parse_outer)?;
            SaladAttrs::parse(attrs)?
        };

        let vis = input.parse::<Visibility>()?;
        let ident = input.parse::<Ident>()?;
        let _ = input.parse::<Token![:]>()?;
        let ty = input.parse::<Type>()?;

        Ok(Self {
            salad,
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
            ..
        } = self;

        tokens.append_all(quote! {
            #( #attrs )*
            #vis #ident: #ty
        })
    }
}
