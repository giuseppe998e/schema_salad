use std::collections::HashMap;

use fxhash::FxBuildHasher;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, Expr, ExprLit, Lit, LitBool, LitStr, Meta,
    MetaNameValue, Token,
};

// Constant attribute keys for SaladAttributes
pub(crate) mod attr_keys {
    pub const IDENTIFIER: &str = "identifier";
    pub const IDENTIFIER_SUBSCOPE: &str = "subscope";
    pub const DEFAULT: &str = "default";
    pub const MAP_KEY: &str = "map_key";
    pub const MAP_PREDICATE: &str = "map_predicate";
    pub const RENAME: &str = "as_str";
    pub const DOCROOT: &str = "root";
}

#[derive(Clone)]
pub(crate) struct SaladAttributes {
    map: HashMap<String, Lit, FxBuildHasher>,
}

impl SaladAttributes {
    /// Indicates whether the field is an identifier
    pub fn identifier(&self) -> bool {
        match self.map.get(attr_keys::IDENTIFIER) {
            Some(Lit::Bool(b)) => b.value(),
            _ => false,
        }
    }

    /// The subscope for idenfication field generation
    pub fn identifier_subscope(&self) -> Option<&LitStr> {
        self.get_lit_str(attr_keys::IDENTIFIER_SUBSCOPE)
    }

    /// The default value for the field if it is not set
    pub fn default_value(&self) -> Option<&Lit> {
        self.map.get(attr_keys::DEFAULT)
    }

    /// The name of the field that is used as the key in the map
    pub fn map_key(&self) -> Option<&LitStr> {
        self.get_lit_str(attr_keys::MAP_KEY)
    }

    /// The name of the field that is used as the map predicate,
    /// in combo with "map_key"
    pub fn map_predicate(&self) -> Option<&LitStr> {
        self.get_lit_str(attr_keys::MAP_PREDICATE)
    }

    /// The string from/to which the object is (de)serialized
    pub fn rename(&self) -> Option<&LitStr> {
        self.get_lit_str(attr_keys::RENAME)
    }

    /// Indicates whether the object should be considered a root
    pub fn document_root(&self) -> bool {
        match self.map.get(attr_keys::DOCROOT) {
            Some(Lit::Bool(b)) => b.value(),
            _ => false,
        }
    }
}

impl SaladAttributes {
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&Lit> {
        self.map.get(key)
    }

    // Utility function for DRY retrieval of LitStr values
    fn get_lit_str(&self, key: &str) -> Option<&LitStr> {
        self.map.get(key).and_then(|lit| match lit {
            Lit::Str(s) => Some(s),
            _ => None,
        })
    }
}

impl TryFrom<&mut Vec<Attribute>> for SaladAttributes {
    type Error = syn::Error;

    fn try_from(attrs: &mut Vec<Attribute>) -> Result<Self, Self::Error> {
        let mut map = HashMap::with_capacity_and_hasher(2, FxBuildHasher::default());

        while let Some(attr) = attrs
            .iter()
            .position(|a| a.path().is_ident("salad"))
            .map(|idx| attrs.remove(idx))
        {
            let Meta::List(list) = attr.meta else {
                return Err(syn::Error::new(attr.span(), "attribute syntax error"));
            };

            for meta in list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
                let (key, value) = parse_meta(meta)?;
                validate_meta(&key, &value)?;
                map.insert(key, value);
            }
        }

        Ok(Self { map })
    }
}

// Parses and returns the key-value pair from Meta
fn parse_meta(meta: Meta) -> syn::Result<(String, Lit)> {
    let key = meta
        .path()
        .get_ident()
        .ok_or_else(|| syn::Error::new(meta.span(), "expected an identifier"))?
        .to_string();
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

// Validates the value type based on the key
fn validate_meta(key: &str, value: &Lit) -> syn::Result<()> {
    match key {
        attr_keys::IDENTIFIER | attr_keys::DOCROOT => match value {
            Lit::Bool(_) => Ok(()),
            _ => Err(syn::Error::new(
                value.span(),
                format!("salad attribute `{}` must be a boolean", key),
            )),
        },
        attr_keys::IDENTIFIER_SUBSCOPE
        | attr_keys::MAP_KEY
        | attr_keys::MAP_PREDICATE
        | attr_keys::RENAME => match value {
            Lit::Str(_) => Ok(()),
            _ => Err(syn::Error::new(
                value.span(),
                format!("salad attribute `{}` must be a string", key),
            )),
        },
        _ => Ok(()), // No validation needed for other keys
    }
}
