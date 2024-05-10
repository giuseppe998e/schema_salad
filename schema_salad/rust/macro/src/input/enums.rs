use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, ToTokens, TokenStreamExt};
use syn::{
    punctuated::Punctuated, token, Attribute, Data, DataEnum, DeriveInput, Field, Fields,
    FieldsUnnamed, Ident, LitStr, Token, Variant, Visibility,
};

use super::{attr_keys, Metadata, SaladAttributes};

// Enum input type
#[derive(Clone)]
pub(crate) struct EnumInput {
    pub salad_attrs: SaladAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub variants: Variants,
    pub seed_ident: Ident,
}

#[derive(Clone)]
pub(crate) enum Variants {
    Tuple(Punctuated<TupleVariant, Token![,]>),
    Unit(Punctuated<UnitVariant, Token![,]>),
}

impl Metadata for EnumInput {
    fn salad_attrs(&self) -> &SaladAttributes {
        &self.salad_attrs
    }

    fn ident(&self) -> &Ident {
        &self.ident
    }

    fn seed_ident(&self) -> &Ident {
        &self.seed_ident
    }
}

impl TryFrom<DeriveInput> for EnumInput {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> Result<Self, Self::Error> {
        let DeriveInput {
            mut attrs,
            vis,
            ident,
            generics,
            data,
            ..
        } = input;

        if !generics.params.is_empty() {
            return Err(syn::Error::new(ident.span(), "generics are not supported"));
        }

        let salad_attrs = SaladAttributes::try_from(&mut attrs)?;

        let variants = {
            let Data::Enum(DataEnum { variants, .. }) = data else {
                unreachable!()
            };

            let mut tuple_variants = Vec::with_capacity(variants.len());
            let mut unit_variants = Vec::with_capacity(variants.len());

            for v in variants.into_iter() {
                if v.fields.is_empty() {
                    let unit_variant = UnitVariant::try_from(v)?;
                    unit_variants.push(unit_variant);
                } else {
                    let tuple_variant = TupleVariant::try_from(v)?;
                    tuple_variants.push(tuple_variant);
                }
            }

            match (tuple_variants.len(), unit_variants.len()) {
                (1.., 0) => Variants::Tuple(Punctuated::from_iter(tuple_variants)),
                (0, 1..) => Variants::Unit(Punctuated::from_iter(unit_variants)),
                (0, 0) => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "enum without variants is not supported",
                    ))
                }
                // MSRV compatibility, it's equal to "(1.., 1..)"
                (_, _) => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "mixed tuple and unit variants enum is not supported",
                    ));
                }
            }
        };

        let seed_ident = format_ident!("__{}Seed", &ident);

        Ok(Self {
            salad_attrs,
            attrs,
            vis,
            ident,
            variants,
            seed_ident,
        })
    }
}

impl ToTokens for Variants {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Variants::Tuple(v) => v.to_tokens(tokens),
            Variants::Unit(v) => v.to_tokens(tokens),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TupleVariant {
    pub salad_attrs: SaladAttributes,
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub field: Field,
}

impl TryFrom<Variant> for TupleVariant {
    type Error = syn::Error;

    fn try_from(input: Variant) -> Result<Self, Self::Error> {
        let Variant {
            mut attrs,
            ident,
            fields,
            ..
        } = input;

        let salad_attrs = SaladAttributes::try_from(&mut attrs)?;

        let field = match fields {
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) if unnamed.len() == 1 => {
                // SAFETY The branch is taken if the vec contains exactly 1 element
                unsafe { unnamed.into_iter().next().unwrap_unchecked() }
            }
            Fields::Unnamed(_) => {
                return Err(syn::Error::new(
                    ident.span(),
                    "multiple unnamed-field variant is not supported",
                ))
            }
            Fields::Named(_) => {
                return Err(syn::Error::new(
                    ident.span(),
                    "named-field variant is not supported",
                ))
            }
            _ => unreachable!(),
        };

        Ok(Self {
            salad_attrs,
            attrs,
            ident,
            field,
        })
    }
}

impl ToTokens for TupleVariant {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.append_all(&self.attrs);
        self.ident.to_tokens(tokens);

        let parents = token::Paren::default();
        parents.surround(tokens, |t| self.field.to_tokens(t));
    }
}

#[derive(Clone)]
pub(crate) struct UnitVariant {
    pub salad_attrs: SaladAttributes,
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub literal: LitStr,
}

impl TryFrom<Variant> for UnitVariant {
    type Error = syn::Error;

    fn try_from(input: Variant) -> Result<Self, Self::Error> {
        let Variant {
            mut attrs, ident, ..
        } = input;

        let salad_attrs = SaladAttributes::try_from(&mut attrs)?;

        let Some(literal) = salad_attrs.rename().cloned() else {
            return Err(syn::Error::new(
                ident.span(),
                format_args!(
                    "unit variants must define the `{}` attribute",
                    attr_keys::RENAME
                ),
            ));
        };

        Ok(Self {
            salad_attrs,
            attrs,
            ident,
            literal,
        })
    }
}

impl ToTokens for UnitVariant {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.append_all(&self.attrs);
        self.ident.to_tokens(tokens);
    }
}
