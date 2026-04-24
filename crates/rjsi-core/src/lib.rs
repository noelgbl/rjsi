mod class;
mod context;
mod global;
pub mod error;
pub mod function;
mod iterator;
mod promise;
mod runtime;
mod source;
mod value;

pub mod engine {
    pub use crate::class::JsClassExt;
    pub use crate::context::{JsContextImpl, JsRawContext};
    pub use crate::runtime::{JsEngine, JsIsolate, JsRuntime};
    pub use crate::value::{
        JsArrayBufferOps, JsArrayOps, JsErrorFactory, JsExceptionThrower, JsObjectOps, JsProxyOps,
        JsTypeOf, JsTypedArrayKind, JsTypedArrayOps, JsValueConversion, JsValueImpl, JsValueType,
    };
}

pub use class::{Class, ClassSetup, JsClass};
pub use context::{JsContext, JsNativeAsyncContext, ThrownValueHandle, ThrownValueStore};
pub use global::Global;
pub use error::{HostError, JsResult, RjsiJSError, illegal_constructor};
pub use function::{Constructor, HostCallback, HostCallbackOnce, RustFunc};
pub use iterator::{
    IntoJsIteratorExt, JsIterator, install_iterator_symbol,
};
pub use promise::Promise;
pub use runtime::{JsEngine, JsIsolate, JsRuntime};
pub use source::{Source, SourceKind};
pub use value::{
    AnyJsTypedArray, FromJsValue, IntoJsValue, JsArray, JsArrayBuffer, JsBytes, JsDate,
    JsException, JsFunc, JsObject, JsProxy, JsSymbol, JsTypedArray, JsTypedArrayKind, JsValue,
    JsValueMapper, JsValueType, JsonToJsValue, PropertyAttributes, PropertyDescriptor, PropertyKey,
    TypedArrayElement, Uint8Clamped,
};

#[doc(hidden)]
pub use engine::{
    JsArrayBufferOps, JsArrayOps, JsClassExt, JsContextImpl, JsErrorFactory,
    JsExceptionThrower, JsObjectOps, JsProxyOps, JsRawContext, JsTypeOf, JsTypedArrayOps,
    JsValueConversion, JsValueImpl,
};

pub mod prelude {
    pub use crate::{
        Class, ClassSetup, FromJsValue, HostCallback, HostCallbackOnce, HostError,
        IntoJsIteratorExt, IntoJsValue, JsArray, JsArrayBuffer, JsArrayBufferOps, JsArrayOps,
        JsBytes, JsClass, JsContext, JsContextImpl, JsDate, JsEngine, JsErrorFactory, JsException,
        JsExceptionThrower, JsFunc, JsIterator, JsNativeAsyncContext, JsObject, JsObjectOps,
        JsProxy, JsProxyOps, JsRawContext, JsResult, JsRuntime, JsIsolate, JsSymbol,
        JsTypedArray, JsTypedArrayOps, JsTypeOf, JsValue, JsValueConversion, JsValueImpl,
        JsValueMapper, Promise, RjsiJSError, Source, SourceKind, install_iterator_symbol,
    };
}
