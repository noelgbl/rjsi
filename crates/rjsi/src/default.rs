#[cfg(all(
    feature = "default-runtime-quickjs",
    any(
        feature = "default-runtime-v8",
        feature = "default-runtime-boa",
        feature = "default-runtime-jsc",
    ),
))]
compile_error!(
    "`default-runtime-quickjs` is mutually exclusive with other `default-runtime-*` features"
);

#[cfg(all(
    feature = "default-runtime-v8",
    any(
        feature = "default-runtime-boa",
        feature = "default-runtime-jsc",
    ),
))]
compile_error!(
    "`default-runtime-v8` is mutually exclusive with other `default-runtime-*` features"
);

#[cfg(all(feature = "default-runtime-boa", feature = "default-runtime-jsc"))]
compile_error!(
    "`default-runtime-boa` and `default-runtime-jsc` are mutually exclusive"
);

#[cfg(all(
    feature = "default-runtime-quickjs",
    not(feature = "default-runtime-v8"),
    not(feature = "default-runtime-boa"),
    not(feature = "default-runtime-jsc"),
))]
pub type DefaultEngine = rjsi_quickjs::QuickJsEngine;

#[cfg(all(
    feature = "default-runtime-v8",
    not(feature = "default-runtime-quickjs"),
    not(feature = "default-runtime-boa"),
    not(feature = "default-runtime-jsc"),
))]
pub type DefaultEngine = rjsi_v8::V8Engine;

#[cfg(all(
    feature = "default-runtime-boa",
    not(feature = "default-runtime-quickjs"),
    not(feature = "default-runtime-v8"),
    not(feature = "default-runtime-jsc"),
))]
pub type DefaultEngine = rjsi_boa::BoaEngine;

#[cfg(all(
    feature = "default-runtime-jsc",
    not(feature = "default-runtime-quickjs"),
    not(feature = "default-runtime-v8"),
    not(feature = "default-runtime-boa"),
))]
pub type DefaultEngine = rjsi_jsc::JscEngine;

#[cfg(all(
    feature = "default-runtime-quickjs",
    not(feature = "default-runtime-v8"),
    not(feature = "default-runtime-boa"),
    not(feature = "default-runtime-jsc"),
))]
pub type DefaultRuntime = rjsi_quickjs::QuickJsRuntime;

#[cfg(all(
    feature = "default-runtime-v8",
    not(feature = "default-runtime-quickjs"),
    not(feature = "default-runtime-boa"),
    not(feature = "default-runtime-jsc"),
))]
pub type DefaultRuntime = rjsi_v8::V8Runtime;

#[cfg(all(
    feature = "default-runtime-boa",
    not(feature = "default-runtime-quickjs"),
    not(feature = "default-runtime-v8"),
    not(feature = "default-runtime-jsc"),
))]
pub type DefaultRuntime = rjsi_boa::BoaRuntime;

#[cfg(all(
    feature = "default-runtime-jsc",
    not(feature = "default-runtime-quickjs"),
    not(feature = "default-runtime-v8"),
    not(feature = "default-runtime-boa"),
))]
pub type DefaultRuntime = rjsi_jsc::JscRuntime;

#[cfg(any(
    all(
        feature = "default-runtime-quickjs",
        not(any(
            feature = "default-runtime-v8",
            feature = "default-runtime-boa",
            feature = "default-runtime-jsc",
        )),
    ),
    all(
        feature = "default-runtime-v8",
        not(any(
            feature = "default-runtime-quickjs",
            feature = "default-runtime-boa",
            feature = "default-runtime-jsc",
        )),
    ),
    all(
        feature = "default-runtime-boa",
        not(any(
            feature = "default-runtime-quickjs",
            feature = "default-runtime-v8",
            feature = "default-runtime-jsc",
        )),
    ),
    all(
        feature = "default-runtime-jsc",
        not(any(
            feature = "default-runtime-quickjs",
            feature = "default-runtime-v8",
            feature = "default-runtime-boa",
        )),
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
        not(any(
            feature = "default-runtime-v8",
            feature = "default-runtime-boa",
            feature = "default-runtime-jsc",
        )),
    ),
    all(
        feature = "default-runtime-v8",
        not(any(
            feature = "default-runtime-quickjs",
            feature = "default-runtime-boa",
            feature = "default-runtime-jsc",
        )),
    ),
    all(
        feature = "default-runtime-boa",
        not(any(
            feature = "default-runtime-quickjs",
            feature = "default-runtime-v8",
            feature = "default-runtime-jsc",
        )),
    ),
    all(
        feature = "default-runtime-jsc",
        not(any(
            feature = "default-runtime-quickjs",
            feature = "default-runtime-v8",
            feature = "default-runtime-boa",
        )),
    ),
))]
pub fn with_default_runtime<R>(f: impl FnOnce(&mut DefaultRuntime) -> R) -> R {
    tls::GLOBAL_RUNTIME.with(|cell| {
        let mut slot = cell.borrow_mut();
        let rt = slot.get_or_insert_with(DefaultRuntime::new);
        f(rt)
    })
}
