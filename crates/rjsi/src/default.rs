#[cfg(all(
    feature = "default-runtime-quickjs",
    feature = "default-runtime-v8"
))]
compile_error!(
    "features `default-runtime-quickjs` and `default-runtime-v8` are mutually exclusive"
);

#[cfg(all(
    feature = "default-runtime-quickjs",
    not(feature = "default-runtime-v8")
))]
pub type DefaultEngine = rjsi_quickjs::QuickJsEngine;

#[cfg(all(
    feature = "default-runtime-v8",
    not(feature = "default-runtime-quickjs")
))]
pub type DefaultEngine = rjsi_v8::V8Engine;

#[cfg(all(
    feature = "default-runtime-quickjs",
    not(feature = "default-runtime-v8")
))]
pub type DefaultRuntime = rjsi_quickjs::QuickJsRuntime;

#[cfg(all(
    feature = "default-runtime-v8",
    not(feature = "default-runtime-quickjs")
))]
pub type DefaultRuntime = rjsi_v8::V8Runtime;

#[cfg(any(
    all(
        feature = "default-runtime-quickjs",
        not(feature = "default-runtime-v8")
    ),
    all(
        feature = "default-runtime-v8",
        not(feature = "default-runtime-quickjs")
    ),
))]
mod tls {
    use super::DefaultRuntime;
    use std::cell::RefCell;

    thread_local! {
        pub(super) static GLOBAL_RUNTIME: RefCell<Option<DefaultRuntime>> = RefCell::new(None);
    }
}

#[cfg(any(
    all(
        feature = "default-runtime-quickjs",
        not(feature = "default-runtime-v8")
    ),
    all(
        feature = "default-runtime-v8",
        not(feature = "default-runtime-quickjs")
    ),
))]
pub fn with_default_runtime<R>(f: impl FnOnce(&mut DefaultRuntime) -> R) -> R {
    tls::GLOBAL_RUNTIME.with(|cell| {
        let mut slot = cell.borrow_mut();
        let rt = slot.get_or_insert_with(DefaultRuntime::new);
        f(rt)
    })
}
