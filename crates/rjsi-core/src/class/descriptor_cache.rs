//! Runtime-agnostic, type-indexed cache for [`ClassDescriptor`].
//!
//! `#[derive(JsClass)]` / `#[js_methods]` should use [`class_descriptor`] so generated code
//! never names a particular engine — only [`crate::Runtime`] and [`super::NativeClass`].

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::Runtime;
use super::ClassDescriptor;

static DESCRIPTORS: LazyLock<Mutex<HashMap<(TypeId, TypeId), usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Return a leaked, process-living [`ClassDescriptor<R>`] for `(T, R)`, building it once with
/// `init`.
///
/// The key is `(TypeId::of::<T>(), TypeId::of::<R>())`, so each native class + engine pair
/// gets a single shared descriptor.
pub fn class_descriptor<R: Runtime, T: super::NativeClass>(
    init: fn() -> ClassDescriptor<R>,
) -> &'static ClassDescriptor<R> {
    let key = (TypeId::of::<T>(), TypeId::of::<R>());
    let mut map = DESCRIPTORS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let addr = *map.entry(key).or_insert_with(|| {
        let d = init();
        let leaked: Box<ClassDescriptor<R>> = Box::new(d);
        let raw = Box::leak(leaked);
        raw as *const ClassDescriptor<R> as usize
    });
    unsafe { &*(addr as *const ClassDescriptor<R>) }
}
