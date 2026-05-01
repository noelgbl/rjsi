#[cfg(feature = "boa")]
pub use rjsi_boa as boa;
#[cfg(feature = "console")]
pub use rjsi_console as console;
pub use rjsi_core::*;
#[cfg(feature = "jsc")]
pub use rjsi_jsc as jsc;
#[cfg(feature = "macros")]
pub use rjsi_macros::{FromJs, IntoJs, JsClass, js_methods};
#[cfg(feature = "quickjs")]
pub use rjsi_quickjs as quickjs;
#[cfg(feature = "v8")]
pub use rjsi_v8 as v8;

#[cfg(any(
    feature = "default-runtime-quickjs",
    feature = "default-runtime-v8",
    feature = "default-runtime-boa",
    feature = "default-runtime-jsc",
))]
pub mod default;
#[cfg(any(
    feature = "default-runtime-quickjs",
    feature = "default-runtime-v8",
    feature = "default-runtime-boa",
    feature = "default-runtime-jsc",
))]
pub use default::*;
