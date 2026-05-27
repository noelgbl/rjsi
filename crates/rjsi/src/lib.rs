#[cfg(feature = "boa")]
pub use rjsi_boa as boa;
#[cfg(feature = "console")]
pub use rjsi_console as console;
#[cfg(feature = "futures")]
pub use rjsi_core::futures;
pub use rjsi_core::*;
#[cfg(feature = "hermes")]
pub use rjsi_hermes as hermes;
#[cfg(feature = "jsc")]
pub use rjsi_jsc as jsc;
#[cfg(feature = "macros")]
pub use rjsi_macros::{
    FromJs, IntoJs, JsClass, NativeState, js_constructor, js_get, js_methods, js_set, js_skip, js_static
};
#[cfg(feature = "quickjs")]
pub use rjsi_quickjs as quickjs;
#[cfg(feature = "v8")]
pub use rjsi_v8 as v8;

#[cfg(any(
    feature = "default-runtime-quickjs",
    feature = "default-runtime-v8",
    feature = "default-runtime-boa",
    feature = "default-runtime-jsc",
    feature = "default-runtime-hermes",
))]
pub mod default;
#[cfg(any(
    feature = "default-runtime-quickjs",
    feature = "default-runtime-v8",
    feature = "default-runtime-boa",
    feature = "default-runtime-jsc",
    feature = "default-runtime-hermes",
))]
pub use default::*;
