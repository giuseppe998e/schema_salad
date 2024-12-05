macro_rules! impl_from_traits {
    (
        ( $ty:ident, $err_ty:ident )
        $( $ident:ident => $subty:ident ),* $(,)?
    ) => {
        $(
            impl From<$subty> for $ty {
                fn from(value: $subty) -> Self {
                    Self::$ident(value)
                }
            }

            impl TryFrom<$ty> for $subty {
                type Error = $err_ty;

                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    match value {
                        $ty::$ident(v) => Ok(v),
                        _ => Err($err_ty::new()),
                    }
                }
            }

            impl<'a> TryFrom<&'a $ty> for &'a $subty {
                type Error = $err_ty;

                fn try_from(value: &'a $ty) -> Result<Self, Self::Error> {
                    match value {
                        $ty::$ident(v) => Ok(v),
                        _ => Err($err_ty::new()),
                    }
                }
            }
        )*
    };
}

pub(crate) use impl_from_traits;
