use enumeration::Variant;
use syn::{
    parse::{Parse, ParseStream},
    token, Error, Ident, Token, Visibility,
};

mod attributes;
pub mod enumeration;
pub mod structure;

pub use self::attributes::Attributes;

use self::{
    enumeration::InputEnum,
    structure::{Field, InputStruct},
};

/// Type sent to a `proc_macro` macro.
pub struct MacroInput {
    pub attrs: Attributes,
    pub vis: Visibility,
    pub ident: Ident,
    pub kind: InputKind,
}

/// Represents different kinds of input types for macro processing.
pub enum InputKind {
    /// Represent a parsable enum type.
    Enum(InputEnum),

    /// Represent a parsable struct type.
    Struct(InputStruct),

    /// Represent a parsable enum type
    /// with unit variants.
    UnitEnum(InputEnum),

    /// Represent a parsable struct type
    /// without fields.
    UnitStruct,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.parse::<Attributes>()?;
        let vis = input.parse::<Visibility>()?;
        let lookahead = input.lookahead1();

        // Parse either struct or enum
        let (ident, kind) = if lookahead.peek(Token![struct]) {
            let _ = input.parse::<Token![struct]>()?;
            parse_struct(input)?
        } else if lookahead.peek(Token![enum]) {
            let _ = input.parse::<Token![enum]>()?;
            parse_enum(input)?
        } else {
            return Err(input.error("expected `struct` or `enum`"));
        };

        Ok(MacroInput {
            attrs,
            vis,
            ident,
            kind,
        })
    }
}

/// Parse a struct definition
fn parse_struct(input: ParseStream) -> syn::Result<(Ident, InputKind)> {
    let ident = input.parse::<Ident>()?;
    let kind = {
        let lookahead = input.lookahead1();

        // Struct with fields
        if lookahead.peek(token::Brace) {
            let content;
            let _ = syn::braced!(content in input);
            let fields = content
                .parse_terminated(Field::parse, Token![,])?
                .into_iter()
                .collect();

            InputKind::Struct(InputStruct { fields })
        }
        // Unit struct
        else if lookahead.peek(Token![;]) {
            let _ = input.parse::<Token![;]>()?;
            InputKind::UnitStruct
        }
        // Unsupported tuple-like struct
        else {
            return Err(Error::new_spanned(
                ident,
                "tuple-like structs are not supported",
            ));
        }
    };

    Ok((ident, kind))
}

/// Parse an enum definition
fn parse_enum(input: ParseStream) -> syn::Result<(Ident, InputKind)> {
    let ident = input.parse::<Ident>()?;
    let kind = {
        let content;
        let _ = syn::parenthesized!(content in input);

        let variants = content
            .parse_terminated(Variant::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();

        let variant_count = variants.len();
        let unit_variant_count = variants.iter().filter(|v| v.ty.is_none()).count();

        match (variant_count, unit_variant_count) {
            (_, 0) => InputKind::Enum(InputEnum { variants }),
            (v, u) if v == u => InputKind::UnitEnum(InputEnum { variants }),
            _ => {
                return Err(Error::new_spanned(
                    ident,
                    "mixed variant types are not supported - all variants must be either unit or value variants",
                ));
            }
        }
    };

    Ok((ident, kind))
}
