use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use compact_str::{CompactString, ToCompactString as _};
use fxhash::FxBuildHasher;
use syn::{
    parse::ParseStream, punctuated::Punctuated, spanned::Spanned as _, Attribute, Expr, ExprLit,
    Lit, LitBool, LitStr, Meta, MetaNameValue, Token,
};

pub const AS_STR: &str = "as_str";
pub const DEFAULT: &str = "default";
pub const DOCROOT: &str = "root";
pub const IDENTIFIER: &str = "identifier";
pub const IDENTIFIER_SUBSCOPE: &str = "subscope";
pub const MAP_KEY: &str = "map_key";
pub const MAP_PREDICATE: &str = "map_predicate";

pub struct SaladAttrs {
    map: HashMap<CompactString, Lit, FxBuildHasher>,
}

impl SaladAttrs {
    pub fn get_str<Q>(&self, key: &Q) -> Option<&LitStr>
    where
        CompactString: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match self.map.get(key) {
            Some(Lit::Str(s)) => Some(s),
            _ => None,
        }
    }
}

impl SaladAttrs {
    pub fn parse_outer(input: ParseStream) -> syn::Result<(Self, Vec<Attribute>)> {
        let mut salad = HashMap::with_capacity_and_hasher(2, FxBuildHasher::default());
        let mut rust = Vec::new();

        while input.peek(Token![#]) {
            let attr = input.call(single_parse_outer)?;

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
                rust.push(attr);
            }
        }

        Ok((Self { map: salad }, rust))
    }
}

fn single_parse_outer(input: ParseStream) -> syn::Result<Attribute> {
    let content;

    Ok(Attribute {
        pound_token: input.parse()?,
        style: syn::AttrStyle::Outer,
        bracket_token: syn::bracketed!(content in input),
        meta: content.parse()?,
    })
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
                    "salad attribute must be a boolean",
                ));
            }
        }
        IDENTIFIER_SUBSCOPE | MAP_KEY | MAP_PREDICATE | AS_STR => {
            if !matches!(value, Lit::Str(_)) {
                return Err(syn::Error::new_spanned(
                    value,
                    "salad attribute must be a string",
                ));
            }
        }
        _ => (), // No validation needed for other keys
    }

    Ok(())
}
