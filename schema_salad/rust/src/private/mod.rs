#![allow(dead_code)]
#![allow(unused_imports)]

pub(crate) mod de;

// DSL Preproccessor feature
#[cfg(feature = "dsl")]
pub(crate) mod dsl;

// Reference counter feature (ON)
#[cfg(feature = "mthread")]
pub type Ref<T> = std::sync::Arc<T>;

// Reference counter feature (OFF)
#[cfg(not(feature = "mthread"))]
pub type Ref<T> = std::rc::Rc<T>;
