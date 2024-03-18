pub(crate) mod de;

// Reference using Arc<T>
#[cfg(feature = "arc")]
pub type Ref<T> = std::sync::Arc<T>;

// Reference using Rc<T>
#[cfg(not(feature = "arc"))]
pub type Ref<T> = std::rc::Rc<T>;
