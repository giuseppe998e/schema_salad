use std::ops::Deref;

use proc_macro2::Span;
use quote::{format_ident, ToTokens};
use syn::{punctuated::Punctuated, Ident, Path, PathSegment};

pub struct TypeIdent {
    ident: Ident,
    pub visitor: Ident,
    pub seed: Path,
}

impl TypeIdent {
    pub fn new(ident: Ident) -> Self {
        let visitor = format_ident!("{}Visitor", ident);
        let seed = Path::from(format_ident!("__{}Seed", ident));

        Self {
            ident,
            visitor,
            seed,
        }
    }

    pub fn new_unit(ident: Ident) -> Self {
        let visitor = format_ident!("{}Visitor", ident);
        let seed = Path {
            leading_colon: Some(Default::default()),
            segments: Punctuated::from_iter([
                PathSegment::from(Ident::new("core", Span::call_site())),
                PathSegment::from(Ident::new("marker", Span::call_site())),
                PathSegment::from(Ident::new("PhantomData", Span::call_site())),
            ]),
        };

        Self {
            ident,
            visitor,
            seed,
        }
    }
}

impl Deref for TypeIdent {
    type Target = Ident;

    fn deref(&self) -> &Self::Target {
        &self.ident
    }
}

impl ToTokens for TypeIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ident.to_tokens(tokens);
    }
}
