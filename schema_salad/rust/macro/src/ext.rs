pub(crate) use self::{strings::StrExt, types::TypeExt};

mod strings {
    pub(crate) trait StrExt {
        fn to_snake_case(&self) -> String;
    }

    impl StrExt for str {
        fn to_snake_case(&self) -> String {
            let mut buf = String::with_capacity(self.len() * 2);
            let mut input_iter = self.chars().peekable();

            while let Some(ch) = input_iter.next() {
                buf.push(ch.to_ascii_lowercase());

                if let Some(next_ch) = input_iter.peek() {
                    if next_ch.is_ascii_uppercase() && ch.is_ascii_lowercase() {
                        buf.push('_')
                    }
                }
            }

            buf
        }
    }
}

mod types {
    use quote::format_ident;
    use syn::{GenericArgument, Ident, PathArguments, Type, TypePath, TypeReference};

    pub(crate) trait TypeExt {
        fn sub_type<'t>(&'t self, filter: Option<&str>) -> Option<&'t Type>;
        fn into_typeref(self) -> Type;
        fn to_variant_ident(&self) -> Ident;
        fn is_salad_primitive(&self) -> bool;
    }

    impl TypeExt for Type {
        fn sub_type<'t>(&'t self, filter: Option<&str>) -> Option<&'t Type> {
            match self {
                Type::Path(TypePath { path, .. }) => {
                    let last_segment = path
                        .segments
                        .last()
                        .filter(|s| matches!(filter, Some(f) if s.ident == f))?;

                    if let PathArguments::AngleBracketed(generics) = &last_segment.arguments {
                        return generics.args.first().and_then(|a| match a {
                            GenericArgument::Type(ty @ Type::Path(_)) => Some(ty),
                            GenericArgument::Type(ty) => ty.sub_type(None),
                            _ => None,
                        });
                    }

                    None
                }
                Type::Array(a) => a.elem.sub_type(None),
                Type::Slice(s) => s.elem.sub_type(None),
                _ => unimplemented!(),
            }
        }

        fn into_typeref(mut self) -> Type {
            match self {
                Type::Path(TypePath { ref mut path, .. }) => {
                    let last_segment = {
                        debug_assert!(path.segments.last().is_some());
                        unsafe { path.segments.last_mut().unwrap_unchecked() }
                    };

                    if last_segment.ident == "Bool"
                        || last_segment.ident == "Int"
                        || last_segment.ident == "Long"
                        || last_segment.ident == "Float"
                        || last_segment.ident == "Double"
                    {
                        return self;
                    } else if last_segment.ident == "Option" {
                        if let PathArguments::AngleBracketed(generics) = &mut last_segment.arguments
                        {
                            if let Some(GenericArgument::Type(subty)) =
                                generics.args.first().cloned()
                            {
                                if !subty.is_salad_primitive() {
                                    let generic_arg = {
                                        debug_assert!(generics.args.first_mut().is_some());
                                        unsafe { generics.args.first_mut().unwrap_unchecked() }
                                    };

                                    *generic_arg = GenericArgument::Type(subty.into_typeref())
                                }

                                return self;
                            }
                        }
                    } else if last_segment.ident == "Box" {
                        if let PathArguments::AngleBracketed(generics) = &last_segment.arguments {
                            if let Some(GenericArgument::Type(subty)) =
                                generics.args.first().cloned()
                            {
                                return if subty.is_salad_primitive() {
                                    subty
                                } else {
                                    Type::Reference(TypeReference {
                                        and_token: Default::default(),
                                        lifetime: None,
                                        mutability: None,
                                        elem: Box::new(subty),
                                    })
                                };
                            }
                        }
                    }

                    Type::Reference(TypeReference {
                        and_token: Default::default(),
                        lifetime: None,
                        mutability: None,
                        elem: Box::new(self),
                    })
                }
                Type::Reference(_) => self,
                _ => unimplemented!(),
            }
        }

        fn to_variant_ident(&self) -> Ident {
            match self {
                Type::Path(TypePath { path, .. }) => {
                    let last_segment = {
                        debug_assert!(path.segments.last().is_some());
                        unsafe { path.segments.last().unwrap_unchecked() }
                    };

                    if let PathArguments::AngleBracketed(generics) = &last_segment.arguments {
                        if let Some(GenericArgument::Type(subty)) = generics.args.first() {
                            return format_ident!(
                                "{}{}",
                                last_segment.ident,
                                subty.to_variant_ident()
                            );
                        }
                    }

                    last_segment.ident.clone()
                }
                Type::Array(a) => format_ident!("{}Array", &a.elem.to_variant_ident()),
                Type::Slice(s) => format_ident!("{}Slice", &s.elem.to_variant_ident()),
                _ => unimplemented!(),
            }
        }

        fn is_salad_primitive(&self) -> bool {
            if let Type::Path(TypePath { path, .. }) = self {
                let last_segment = {
                    debug_assert!(path.segments.last().is_some());
                    unsafe { path.segments.last().unwrap_unchecked() }
                };

                return last_segment.ident == "Bool"
                    || last_segment.ident == "Int"
                    || last_segment.ident == "Long"
                    || last_segment.ident == "Float"
                    || last_segment.ident == "Double";
            }

            false
        }
    }
}
