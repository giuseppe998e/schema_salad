mod attrs;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Data, DataEnum, DataStruct, DeriveInput, Field, Fields, FieldsNamed,
    FieldsUnnamed, Ident, Token, Type, Variant, Visibility,
};

pub(crate) use self::attrs::{
    MacroAttributes, SALAD_ATTR_AS_STR, SALAD_ATTR_DEFAULT, SALAD_ATTR_ID, SALAD_ATTR_MAP_KEY,
    SALAD_ATTR_MAP_PREDICATE, SALAD_ATTR_ROOT, SALAD_ATTR_SUBSCOPE,
};
use crate::util::{StrExt, TypeExt};

pub(crate) type PunctuatedFields = Punctuated<StructField, Token![,]>;
pub(crate) type PunctuatedVariants = Punctuated<EnumVariant, Token![,]>;

#[derive(Clone)]
pub(crate) enum MacroInput {
    Struct(InputStruct),
    Enum(InputEnum),
    Unit(InputUnit),
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = DeriveInput::parse(input)?;
        match &input.data {
            Data::Struct(s) if matches!(s.fields, Fields::Unit) => {
                InputUnit::try_from(input).map(Self::Unit)
            }
            Data::Struct(_) => InputStruct::try_from(input).map(Self::Struct),
            Data::Enum(_) => InputEnum::try_from(input).map(Self::Enum),
            Data::Union(_) => Err(syn::Error::new(input.span(), "unions are not supported.")),
        }
    }
}

// Struct
#[derive(Clone)]
pub(crate) struct InputStruct {
    pub salad_attrs: MacroAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub fields: PunctuatedFields,
    pub seed_ident: Ident,
    pub value_ident: Ident,
}

impl TryFrom<DeriveInput> for InputStruct {
    type Error = syn::Error;

    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        let DeriveInput {
            attrs,
            vis,
            ident,
            generics,
            data,
        } = value;

        if !generics.params.is_empty() {
            return Err(syn::Error::new(ident.span(), "generics are not supported."));
        }

        let mut attrs = attrs;
        let salad_attrs = MacroAttributes::try_from(&mut attrs)?;

        let fields = {
            let fields = match data {
                Data::Struct(DataStruct {
                    fields: Fields::Named(FieldsNamed { named, .. }),
                    ..
                }) => named,
                Data::Struct(DataStruct {
                    fields: Fields::Unnamed(..),
                    ..
                }) => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "unnamed fields structs are not supported.",
                    ))
                }
                _ => unreachable!(),
            };

            fields
                .into_iter()
                .map(StructField::try_from)
                .collect::<syn::Result<Punctuated<_, Token![,]>>>()?
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
pub(crate) struct StructField {
    pub salad_attrs: MacroAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub ty: Type,
    pub literal: String,
    pub variant_ident: Ident,
}

impl TryFrom<Field> for StructField {
    type Error = syn::Error;

    fn try_from(value: Field) -> Result<Self, Self::Error> {
        let Field {
            attrs,
            vis,
            ident,
            ty,
            ..
        } = value;

        let Some(ident) = ident else {
            return Err(syn::Error::new(
                ty.span(),
                "unnamed fields are not supported.",
            ));
        };

        let mut attrs = attrs;
        let salad_attrs = MacroAttributes::try_from(&mut attrs)?;

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
            vis,
            ident,
            ty,
            literal,
            variant_ident,
        })
    }
}

impl ToTokens for StructField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.append_all(&self.attrs);
        self.vis.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        token::Colon::default().to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

// Enum
#[derive(Clone)]
pub(crate) struct InputEnum {
    pub salad_attrs: MacroAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub variants: PunctuatedVariants,
    pub seed_ident: Ident,
}

impl TryFrom<DeriveInput> for InputEnum {
    type Error = syn::Error;

    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        let DeriveInput {
            attrs,
            vis,
            ident,
            generics,
            data,
            ..
        } = value;

        if !generics.params.is_empty() {
            return Err(syn::Error::new(ident.span(), "generics are not supported."));
        }

        let mut attrs = attrs;
        let salad_attrs = MacroAttributes::try_from(&mut attrs)?;

        let variants = {
            let variants = match data {
                Data::Enum(DataEnum { variants, .. }) => variants,
                _ => unreachable!(),
            };

            variants
                .into_iter()
                .map(EnumVariant::try_from)
                .collect::<syn::Result<Punctuated<_, Token![,]>>>()?
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

#[derive(Clone)]
pub(crate) struct EnumVariant {
    #[allow(dead_code)]
    pub salad_attrs: MacroAttributes,
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub field: Option<Field>,
}

impl TryFrom<Variant> for EnumVariant {
    type Error = syn::Error;

    fn try_from(value: Variant) -> Result<Self, Self::Error> {
        let Variant {
            attrs,
            ident,
            fields,
            ..
        } = value;

        let mut attrs = attrs;
        let salad_attrs = MacroAttributes::try_from(&mut attrs)?;

        let field = match fields {
            Fields::Unit => None,
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) if unnamed.len() == 1 => {
                unnamed.into_iter().next()
            }
            Fields::Unnamed(_) => {
                return Err(syn::Error::new(
                    ident.span(),
                    "multiple unnamed-field variant is not supported.",
                ))
            }
            Fields::Named(_) => {
                return Err(syn::Error::new(
                    ident.span(),
                    "named-field variant is not supported.",
                ))
            }
        };

        Ok(Self {
            salad_attrs,
            attrs,
            ident,
            field,
        })
    }
}

impl ToTokens for EnumVariant {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.append_all(&self.attrs);
        self.ident.to_tokens(tokens);

        if let Some(field) = self.field.as_ref() {
            let parents = token::Paren::default();
            parents.surround(tokens, |t| field.to_tokens(t));
        }
    }
}

// Union
#[derive(Clone)]
pub(crate) struct InputUnit {
    pub salad_attrs: MacroAttributes,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
}

impl TryFrom<DeriveInput> for InputUnit {
    type Error = syn::Error;

    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        let DeriveInput {
            attrs, vis, ident, ..
        } = value;

        let mut attrs = attrs;
        let salad_attrs = MacroAttributes::try_from(&mut attrs)?;

        return Ok(Self {
            salad_attrs,
            attrs,
            vis,
            ident,
        });
    }
}
