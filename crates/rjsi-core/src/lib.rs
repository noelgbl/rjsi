pub mod callback;
pub mod capabilities;
pub mod context;
pub mod error;
pub mod persistent;
pub mod runtime;
pub mod scope;
pub mod convert;
pub mod value;

pub use callback::{Args, Callback, FromJsTuple, bind};
pub use capabilities::ScopeArrayBuffer;
pub use context::ContextLike;
pub use error::{
    E_ABORT, E_ERROR, E_INTERNAL, E_INVALID_ARG, E_INVALID_DATA, E_INVALID_STATE, E_IO,
    E_NOT_SUPPORTED, E_OUT_OF_RANGE, E_TYPE, HostError, RjsiError,
};
pub use persistent::{Global, PersistentLike};
pub use runtime::Runtime;
pub use scope::{ScopeLike, TryCatchResult};
pub use convert::{FromJs, IntoJs, ZeroCopyBuf};
pub use value::{JsFunction, ValueLike};

pub mod prelude {
    pub use crate::{
        Args, ContextLike, FromJs, FromJsTuple, JsFunction, Global, HostError, IntoJs,
        PersistentLike, Runtime, ScopeArrayBuffer, ScopeLike, TryCatchResult, ValueLike,
        ZeroCopyBuf, bind,
    };
}
