use crate::{Args, Runtime};
use super::NativeRef;

/// Complete compile-time description of a native class.
/// All fields are `&'static` — no heap allocation.
/// One instance exists per `(Type, Runtime)` pair, stored in a [`once_cell::sync::OnceCell`].
pub struct ClassDescriptor<R: Runtime> {
    /// Human-readable name used by engines for `[Symbol.toStringTag]` / debugging.
    pub name: &'static str,
    /// Called when `new MyClass(...)` is invoked from JS. `None` → not constructible from JS.
    pub constructor: Option<ConstructorFn<R>>,
    pub methods: &'static [MethodDescriptor<R>],
    pub statics: &'static [MethodDescriptor<R>],
    pub accessors: &'static [AccessorDescriptor<R>],
    pub symbols: &'static [SymbolDescriptor<R>],
    /// Called when the engine GC collects a wrapped object.
    pub finalizer: FinalizerFn,
}

/// Called when `new MyClass(...)` is invoked from JS.
/// Must call `scope.wrap_native(instance)` (or equivalent) and return the result when appropriate.
pub type ConstructorFn<R> = for<'s> fn(
    scope: &mut <R as Runtime>::Scope<'s, 's>,
    args: Args<'s, R>,
) -> Result<<R as Runtime>::Value<'s>, <R as Runtime>::Error>;

/// Descriptor for an instance method on the prototype.
pub struct MethodDescriptor<R: Runtime> {
    /// The property name on the JS prototype (`decode`, `toString`, …).
    pub name: &'static str,
    /// Expected argument count — used by engines for `Function.length`.
    pub arity: u32,
    /// The bridge function. `this_ref` is the unwrapped Rust value.
    pub call: InstanceMethodFn<R>,
}

/// An instance method bridge.
/// `this_ref` carries a mutable reference to the Rust value inside the JS object.
pub type InstanceMethodFn<R> = for<'s> fn(
    scope: &mut <R as Runtime>::Scope<'s, 's>,
    this_ref: NativeRef<'s>,
    args: Args<'s, R>,
) -> Result<<R as Runtime>::Value<'s>, <R as Runtime>::Error>;

/// Getter/setter pair on the prototype.
pub struct AccessorDescriptor<R: Runtime> {
    pub name: &'static str,
    pub getter: Option<AccessorFn<R>>,
    pub setter: Option<AccessorFn<R>>,
}

pub type AccessorFn<R> = for<'s> fn(
    scope: &mut <R as Runtime>::Scope<'s, 's>,
    this_ref: NativeRef<'s>,
    arg: Option<<R as Runtime>::Value<'s>>,
) -> Result<<R as Runtime>::Value<'s>, <R as Runtime>::Error>;

/// Well-known symbol method (e.g. `Symbol.iterator`).
pub struct SymbolDescriptor<R: Runtime> {
    pub symbol: WellKnownSymbol,
    pub call: InstanceMethodFn<R>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WellKnownSymbol {
    Iterator,
    AsyncIterator,
    ToPrimitive,
    ToStringTag,
    HasInstance,
}

/// Called by the engine GC when a wrapped object is collected.
/// Receives the raw `Box<T>` pointer stored in slot 0.
///
/// # Safety
/// The pointer must have been produced by `Box::into_raw::<T>()`.
/// The engine guarantees this function is called at most once per object.
pub type FinalizerFn = unsafe fn(*mut std::ffi::c_void);
