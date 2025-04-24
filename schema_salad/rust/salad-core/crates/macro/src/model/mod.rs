use syn::{
    parse::{Parse, ParseStream},
    token, Attribute, Error, Ident, Token, Type, Visibility,
};

pub mod attributes;
mod ident;

pub use self::{attributes::SaladAttrs, ident::TypeIdent};

/// Represents the input to the macro, including metadata, and kind.
pub struct MacroInput {
    pub salad: SaladAttrs,
    pub meta: InputMetadata,
    pub kind: InputKind,
}

/// Contains metadata for the input, such as attributes,
/// visibility, and identifier.
pub struct InputMetadata {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: TypeIdent,
}

/// Enum representing the kind of input, which can be an enum, struct,
/// unit enum, or unit struct.
pub enum InputKind {
    /// Represent a parsable enum type.
    Enum(Vec<Variant>),

    /// Represent a parsable struct type.
    Struct(Vec<Field>),

    /// Represent a parsable enum type
    /// with unit variants.
    UnitEnum(Vec<Variant>),

    /// Represent a parsable struct type
    /// without fields.
    UnitStruct,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (salad, attrs) = input.call(SaladAttrs::parse_outer)?;
        let vis = input.parse::<Visibility>()?;

        // Parse either struct or enum
        let (ident, kind) = {
            let lookahead = input.lookahead1();

            if lookahead.peek(Token![struct]) {
                let _ = input.parse::<Token![struct]>()?;
                parse_struct(input)?
            } else if lookahead.peek(Token![enum]) {
                let _ = input.parse::<Token![enum]>()?;
                parse_enum(input)?
            } else {
                return Err(input.error("expected `struct` or `enum`"));
            }
        };

        let meta = InputMetadata { attrs, vis, ident };
        Ok(MacroInput { salad, meta, kind })
    }
}

/// Parses a struct from the input stream.
///
/// This function handles both regular structs with fields and unit structs.
/// It returns a tuple containing the identifier of the struct and its kind.
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

        Ok((TypeIdent::new(ident), InputKind::Struct(fields)))
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

/// Parses an enum from the input stream.
///
/// This function handles both regular enums with variants and unit enums.
/// It returns a tuple containing the identifier of the enum and its kind.
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
        0 => Ok((TypeIdent::new(ident), InputKind::Enum(variants))),
        u if u == variants.len() => Ok((TypeIdent::new_unit(ident), InputKind::UnitEnum(variants))),
        _ => Err(Error::new_spanned(
            ident,
            "all variants must be either unit or value variants",
        )),
    }
}

/// Represents a field in a struct, including salad attributes,
/// rust attributes, visibility, identifier, and type.
pub struct Field {
    pub salad: SaladAttrs,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub ty: Type,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (salad, attrs) = input.call(SaladAttrs::parse_outer)?;
        let vis = input.parse::<Visibility>()?;
        let ident = input.parse::<Ident>()?;
        let _ = input.parse::<Token![:]>()?;
        let ty = input.parse::<Type>()?;

        Ok(Self {
            salad,
            attrs,
            vis,
            ident,
            ty,
        })
    }
}

/// Represents a variant in an enum, including salad attributes,
/// rust attributes, identifier, and an optional type.
pub struct Variant {
    pub salad: SaladAttrs,
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub ty: Option<Type>,
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (salad, attrs) = input.call(SaladAttrs::parse_outer)?;
        let ident = input.parse::<Ident>()?;
        let ty = {
            let lookahead = input.lookahead1();

            if lookahead.peek(token::Paren) {
                let content;
                let _ = syn::parenthesized!(content in input);
                content.parse::<Type>().map(Some)?
            } else {
                None
            }
        };

        Ok(Self {
            salad,
            attrs,
            ident,
            ty,
        })
    }
}
