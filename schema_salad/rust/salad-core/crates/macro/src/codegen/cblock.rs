use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt as _};

/// Intermediate representation for generating a const block with salad_core imports.
///
/// This struct accumulates tokens that will be wrapped in a const block with
/// the necessary `extern crate` and `use` declarations for salad_core.
#[derive(Default)]
pub struct ConstBlockIr {
    body: TokenStream2,
}

impl<T: ToTokens> Extend<T> for ConstBlockIr {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for tok in iter {
            tok.to_tokens(&mut self.body);
        }
    }
}

impl<T: ToTokens> FromIterator<T> for ConstBlockIr {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

impl ToTokens for ConstBlockIr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let body = &self.body;

        tokens.append_all(quote! {
            const _: () = {
                extern crate salad_core as __core;
                use __core::__private as __core__priv;

                #body
            };
        });
    }
}
