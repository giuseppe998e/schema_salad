#![allow(unused)] // TODO remove

pub(crate) mod attrs;
mod enums;
mod structs;

use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Data, DeriveInput, Fields, Ident, LitStr, Visibility,
};

pub(crate) use self::{
    attrs::{attr_keys, SaladAttributes},
    enums::{EnumInput, TupleVariant, UnitVariant, Variants},
    structs::{Field, StructInput},
};

// Macro input trait
pub(crate) trait Metadata {
    fn salad_attrs(&self) -> &SaladAttributes;
    fn ident(&self) -> &Ident;
    fn seed_ident(&self) -> &Ident;
}

// Macro input types
#[derive(Clone)]
pub(crate) enum MacroInput {
    Struct(StructInput),
    Enum(EnumInput),
    Unit(UnitInput),
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = DeriveInput::parse(input)?;

        match &input.data {
            Data::Struct(s) => match &s.fields {
                Fields::Unit => UnitInput::try_from(input).map(Self::Unit),
                _ => StructInput::try_from(input).map(Self::Struct),
            },
            Data::Enum(_) => EnumInput::try_from(input).map(Self::Enum),
            Data::Union(_) => Err(syn::Error::new(input.span(), "union type is not supported")),
        }
    }
}

// Unit input type
#[derive(Clone)]
pub(crate) struct UnitInput {
    pub salad_attrs: SaladAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub literal: LitStr,
}

impl Metadata for UnitInput {
    fn salad_attrs(&self) -> &SaladAttributes {
        &self.salad_attrs
    }

    fn ident(&self) -> &Ident {
        &self.ident
    }

    fn seed_ident(&self) -> &Ident {
        &self.ident
    }
}

impl TryFrom<DeriveInput> for UnitInput {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> Result<Self, Self::Error> {
        let DeriveInput {
            mut attrs,
            vis,
            ident,
            ..
        } = input;

        let salad_attrs = SaladAttributes::try_from(&mut attrs)?;

        let Some(literal) = salad_attrs.rename().cloned() else {
            return Err(syn::Error::new(
                ident.span(),
                format_args!(
                    "unit types must define the `{}` attribute",
                    attr_keys::RENAME
                ),
            ));
        };

        Ok(Self {
            salad_attrs,
            attrs,
            vis,
            ident,
            literal,
        })
    }
}
