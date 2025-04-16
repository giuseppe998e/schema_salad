use syn::{
    parse::{Parse, ParseStream},
    token, Attribute, Error, Ident, Token, Visibility,
};

pub mod attributes;
mod enumeration;
mod ident;
mod structure;

pub use self::{
    attributes::SaladAttrs,
    enumeration::{InputEnum, Variant},
    ident::TypeIdent,
    structure::{Field, InputStruct},
};

/// Type sent to a `proc_macro` macro.
pub struct MacroInput {
    pub salad: SaladAttrs,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: TypeIdent,
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
        let (salad, attrs) = {
            let attrs = input.call(Attribute::parse_outer)?;
            SaladAttrs::parse(attrs)?
        };

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
            salad,
            attrs,
            vis,
            ident,
            kind,
        })
    }
}

/// Parse a struct definition
fn parse_struct(input: ParseStream) -> syn::Result<(TypeIdent, InputKind)> {
    let ident = input.parse::<Ident>()?;
    let lookahead = input.lookahead1();

    // Struct with fields
    if lookahead.peek(token::Brace) {
        let content;
        let _ = syn::braced!(content in input);
        let fields = content
            .parse_terminated(Field::parse, Token![,])?
            .into_iter()
            .collect();

        Ok((
            TypeIdent::new(ident),
            InputKind::Struct(InputStruct { fields }),
        ))
    }
    // Unit struct
    else if lookahead.peek(Token![;]) {
        let _ = input.parse::<Token![;]>()?;
        Ok((TypeIdent::new_unit(ident), InputKind::UnitStruct))
    }
    // Unsupported tuple-like struct
    else {
        Err(Error::new_spanned(
            ident,
            "tuple-like structs are not supported",
        ))
    }
}

/// Parse an enum definition
fn parse_enum(input: ParseStream) -> syn::Result<(TypeIdent, InputKind)> {
    let ident = input.parse::<Ident>()?;

    let content;
    let _ = syn::braced!(content in input);

    let variants = content
        .parse_terminated(Variant::parse, Token![,])?
        .into_iter()
        .collect::<Vec<_>>();

    let unit_variant_count = variants.iter().filter(|v| v.ty.is_none()).count();
    match unit_variant_count {
        0 => Ok((
            TypeIdent::new(ident),
            InputKind::Enum(InputEnum { variants }),
        )),
        u if u == variants.len() => Ok((
            TypeIdent::new_unit(ident),
            InputKind::UnitEnum(InputEnum { variants }),
        )),
        _ => Err(Error::new_spanned(
            ident,
            "all variants must be either unit or value variants",
        )),
    }
}
