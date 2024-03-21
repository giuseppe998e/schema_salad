pub mod core;
pub(crate) mod de;

// Reference to structs using Arc<T> or Rc<T>
// according to the feature.
#[cfg(feature = "arc")]
pub type Ref<T> = std::sync::Arc<T>;
#[cfg(not(feature = "arc"))]
pub type Ref<T> = std::rc::Rc<T>;

// Generated code

