use quote::format_ident;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, Data, DataStruct, DeriveInput,
    Field as SynField, Fields, FieldsNamed, Ident, Token, Type, Visibility,
};

use super::{Metadata, SaladAttributes};
use crate::ext::{StrExt, TypeExt};

// Struct input type
#[derive(Clone)]
pub(crate) struct StructInput {
    pub salad_attrs: SaladAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub fields: Punctuated<Field, Token![,]>,
    pub seed_ident: Ident,
    pub value_ident: Ident,
}

impl Metadata for StructInput {
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

impl TryFrom<DeriveInput> for StructInput {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> Result<Self, Self::Error> {
        let DeriveInput {
            mut attrs,
            vis,
            ident,
            generics,
            data,
        } = input;

        if !generics.params.is_empty() {
            return Err(syn::Error::new(ident.span(), "generics are not supported"));
        }

        let salad_attrs = SaladAttributes::try_from(&mut attrs)?;

        let fields = match data {
            Data::Struct(DataStruct {
                fields: Fields::Named(FieldsNamed { named, .. }),
                ..
            }) => named
                .into_iter()
                .map(Field::try_from)
                .collect::<syn::Result<Punctuated<_, Token![,]>>>()?,
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(..),
                ..
            }) => {
                return Err(syn::Error::new(
                    ident.span(),
                    "unnamed fields structs are not supported",
                ))
            }
            _ => unreachable!(),
        };

        let seed_ident = format_ident!("__{}Seed", &ident);
        let value_ident = format_ident!("__{}Value", &ident);

        Ok(Self {
            salad_attrs,
            attrs,
            vis,
            ident,
            fields,
            seed_ident,
            value_ident,
        })
    }
}

#[derive(Clone)]
pub(crate) struct Field {
    pub salad_attrs: SaladAttributes,
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub ty: Type,
    pub literal: String,
    pub variant_ident: Ident,
}

impl TryFrom<SynField> for Field {
    type Error = syn::Error;

    fn try_from(input: SynField) -> Result<Self, Self::Error> {
        let SynField {
            mut attrs,
            ident,
            ty,
            ..
        } = input;

        let Some(ident) = ident else {
            return Err(syn::Error::new(
                ty.span(),
                "unnamed fields are not supported",
            ));
        };

        let salad_attrs = SaladAttributes::try_from(&mut attrs)?;

        let (ident, literal) = {
            let literal = ident.to_string();
            match literal.strip_prefix("r#") {
                Some(substr) => {
                    let mut ident_substr = substr.to_owned();
                    ident_substr.push('_');
                    (Ident::new(&ident_substr, ident.span()), substr.to_owned())
                }
                None => (Ident::new(&literal.to_snake_case(), ident.span()), literal),
            }
        };

        let variant_ty = ty.sub_type(Some("Option")).unwrap_or(&ty);
        let variant_ident = variant_ty.to_variant_ident();

        Ok(Self {
            salad_attrs,
            attrs,
            ident,
            ty,
            literal,
            variant_ident,
        })
    }
}
