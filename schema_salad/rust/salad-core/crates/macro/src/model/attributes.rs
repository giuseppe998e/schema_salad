use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use compact_str::{CompactString, ToCompactString as _};
use fxhash::FxBuildHasher;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned as _,
    Attribute, Expr, ExprLit, Lit, LitBool, LitStr, Meta, MetaNameValue, Token,
};

pub const AS_STR: &str = "as_str";
pub const DEFAULT: &str = "default";
pub const DOCROOT: &str = "root";
pub const IDENTIFIER: &str = "identifier";
pub const IDENTIFIER_SUBSCOPE: &str = "subscope";
pub const MAP_KEY: &str = "map_key";
pub const MAP_PREDICATE: &str = "map_predicate";

pub struct Attributes {
    salad: HashMap<CompactString, Lit, FxBuildHasher>,
    common: Vec<Attribute>,
}

impl Attributes {
    pub fn get_str<Q>(&self, key: &Q) -> Option<&LitStr>
    where
        CompactString: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.salad.get(key).and_then(|l| match l {
            Lit::Str(s) => Some(s),
            _ => None,
        })
    }
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;

        let mut salad = HashMap::with_hasher(FxBuildHasher::default());
        let mut common = Vec::with_capacity(attrs.len());

        for attr in attrs {
            if attr.path().is_ident("salad") {
                let Meta::List(list) = attr.meta else {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "invalid salad attribute syntax",
                    ));
                };

                for meta in list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
                    let (key, value) = parse_meta(meta)?;
                    validate_meta(&key, &value)?;
                    salad.insert(key, value);
                }
            } else {
                common.push(attr);
            }
        }

        Ok(Self { salad, common })
    }
}

fn parse_meta(meta: Meta) -> syn::Result<(CompactString, Lit)> {
    let key = meta
        .path()
        .get_ident()
        .ok_or_else(|| syn::Error::new(meta.span(), "expected an identifier"))?
        .to_compact_string();

    let value = match meta {
        Meta::Path(_) => Lit::Bool(LitBool::new(true, meta.span())),
        Meta::NameValue(MetaNameValue {
            value: Expr::Lit(ExprLit { lit, .. }),
            ..
        }) => lit,
        _ => return Err(syn::Error::new(meta.span(), "unsupported metadata format")),
    };

    Ok((key, value))
}

fn validate_meta(key: &str, value: &Lit) -> syn::Result<()> {
    match key {
        IDENTIFIER | DOCROOT => {
            if !matches!(value, Lit::Bool(_)) {
                return Err(syn::Error::new_spanned(
                    value,
                    format!("salad attribute `{key}` must be a boolean"),
                ));
            }
        }
        IDENTIFIER_SUBSCOPE | MAP_KEY | MAP_PREDICATE | AS_STR => {
            if !matches!(value, Lit::Str(_)) {
                return Err(syn::Error::new_spanned(
                    value,
                    format!("salad attribute `{key}` must be a string"),
                ));
            }
        }
        _ => {} // No validation needed for other keys
    }

    Ok(())
}

impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.append_all(self.common.iter());
    }
}
