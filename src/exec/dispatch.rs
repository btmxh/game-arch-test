use trait_set::trait_set;

use crate::context::event::EventDispatchContext;

trait_set! {
    pub trait NonSendDispatch<T> = FnOnce(T) + 'static;
    pub trait Dispatch<T> = NonSendDispatch<T> + Send + 'static;
    pub trait EventDispatch = for <'a> NonSendDispatch<EventDispatchContext<'a>>;
}
