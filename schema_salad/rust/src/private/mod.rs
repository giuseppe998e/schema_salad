#![allow(dead_code)]
#![allow(unused_imports)]

pub(crate) mod de;

#[cfg(feature = "arc")]
pub type Ref<T> = std::sync::Arc<T>;

#[cfg(not(feature = "arc"))]
pub type Ref<T> = std::rc::Rc<T>;
