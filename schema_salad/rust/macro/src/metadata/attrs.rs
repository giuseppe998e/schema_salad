use std::collections::HashMap;

use fxhash::FxBuildHasher;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, Expr, ExprLit, Lit, LitBool, LitStr, Meta,
    MetaNameValue, Token,
};

pub(crate) const SALAD_ATTR_AS_STR: &str = "as_str";
pub(crate) const SALAD_ATTR_DEFAULT: &str = "default";
pub(crate) const SALAD_ATTR_ID: &str = "identifier";
pub(crate) const SALAD_ATTR_MAP_KEY: &str = "map_key";
pub(crate) const SALAD_ATTR_MAP_PREDICATE: &str = "map_predicate";
pub(crate) const SALAD_ATTR_ROOT: &str = "root";
pub(crate) const SALAD_ATTR_SUBSCOPE: &str = "subscope";

#[derive(Clone)]
pub(crate) struct MacroAttributes {
    map: HashMap<String, Lit, FxBuildHasher>,
}

impl MacroAttributes {
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn contains_and_is_true(&self, key: &str) -> bool {
        match self.map.get(key) {
            Some(Lit::Bool(LitBool { value, .. })) => *value,
            _ => false,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Lit> {
        self.map.get(key)
    }

    pub fn get_string(&self, key: &str) -> syn::Result<Option<&LitStr>> {
        match self.map.get(key) {
            Some(Lit::Str(s)) => Ok(Some(s)),
            Some(lit) => Err(syn::Error::new(lit.span(), "A string value was expected.")),
            _ => Ok(None),
        }
    }
}

impl TryFrom<&mut Vec<Attribute>> for MacroAttributes {
    type Error = syn::Error;

    fn try_from(attrs: &mut Vec<Attribute>) -> Result<Self, Self::Error> {
        let mut map = HashMap::with_capacity_and_hasher(2, FxBuildHasher::default());

        while let Some(idx) = attrs.iter().position(|a| a.path().is_ident("salad")) {
            let attr = attrs.remove(idx);
            let Meta::List(list) = attr.meta else {
                return Err(syn::Error::new(attr.span(), "Attribute syntax error."));
            };

            for meta in list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
                let key = tryfrom_util::get_key(&meta)?;
                let value = tryfrom_util::get_value(meta)?;
                map.insert(key, value);
            }
        }

        Ok(Self { map })
    }
}

// `TryFrom<..>` util methods
mod tryfrom_util {
    use super::*;

    pub(super) fn get_key(meta: &Meta) -> syn::Result<String> {
        match meta.path().get_ident().map(ToString::to_string) {
            Some(k) => Ok(k),
            None => Err(syn::Error::new(
                meta.span(),
                "Named arguments only are allowed.",
            )),
        }
    }

    pub(super) fn get_value(meta: Meta) -> syn::Result<Lit> {
        match meta {
            Meta::Path(m) => Ok(Lit::Bool(LitBool {
                value: true,
                span: m.span(),
            })),
            Meta::NameValue(MetaNameValue {
                value: Expr::Lit(ExprLit { lit, .. }),
                ..
            }) => Ok(lit),
            _ => Err(syn::Error::new(
                meta.span(),
                "List-type arguments are not supported.",
            )),
        }
    }
}