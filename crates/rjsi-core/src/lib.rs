pub mod error;
pub mod function;
mod global;
mod runtime;
mod source;
mod value;

pub mod convert;

pub mod engine {
    pub use crate::function::{
        FromHostArgs, FromJs, HostArgs, HostFunction, IntoHostReturn, IntoJs, ParamsAccessor, RustFunc, SliceHostArgs, TypedHostFunction
    };
    pub use crate::global::{Global, JsGlobalHandle};
    pub use crate::runtime::{JsEngine, JsRuntime, JsScope};
    pub use crate::value::{JsValueType, PropertyAttributes};
}

pub use error::{HostError, JsResult, RjsiJSError};
pub use function::{
    ArrayHostArgs, FromHostArgs, FromJs, HostArgs, HostFunction, IntoHostReturn, IntoJs, ParamsAccessor, RustFunc, SliceHostArgs, TypedHostFunction
};
pub use global::{Global, JsGlobalHandle};
pub use runtime::{JsEngine, JsRuntime, JsScope};
pub use source::{Source, SourceKind};
pub use value::{JsValueType, PropertyAttributes};

pub mod prelude {
    pub use crate::{
        FromHostArgs, FromJs, Global, HostArgs, HostError, HostFunction, IntoHostReturn, IntoJs, JsEngine, JsGlobalHandle, JsResult, JsRuntime, JsValueType, ParamsAccessor, PropertyAttributes, RjsiJSError, RustFunc, SliceHostArgs, Source, SourceKind, TypedHostFunction
    };
}
