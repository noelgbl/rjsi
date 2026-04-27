//! Minimal in-process runtime for unit tests (classes, macros).
//!
//! Constructor calls use a targeted lifetime adjustment because [`MockScope`]'s `'p`
//! parameter is phantom-only (`Arc` storage does not depend on it).
#![allow(clippy::type_complexity)]
#![allow(clippy::arc_with_non_send_sync)]

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::{
    error::E_TYPE,
    Args, ClassRegistry, ConstructorFn, ContextLike, HostError, JsFunction, NativeClass,
    PersistentLike, Runtime, ScopeLike, TryCatchResult, ValueLike,
};

/// Mock engine tag.
pub struct MockRuntime;

/// Handle to the mock JS context.
#[derive(Clone)]
pub struct MockContext {
    inner: Arc<MockInner>,
}

struct MockInner {
    registered: RefCell<HashMap<TypeId, &'static crate::ClassDescriptor<MockRuntime>>>,
    global: Arc<RefCell<HashMap<String, MockValue>>>,
}

#[derive(Debug, Clone)]
pub struct MockError {
    message: String,
}

impl MockError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MockError {}

impl From<HostError> for MockError {
    fn from(value: HostError) -> Self {
        Self::new(value.to_string())
    }
}

/// JS-like value for the mock engine.
#[derive(Clone)]
pub enum MockValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(Arc<RefCell<HashMap<String, MockValue>>>),
    Function(Arc<dyn Fn(&mut MockScope<'_, '_>, Args<'_, MockRuntime>) -> Result<MockValue, MockError> + Send + Sync>),
    Ctor(Option<ConstructorFn<MockRuntime>>),
    NativeObject {
        type_id: TypeId,
        ptr: Arc<Mutex<*mut std::ffi::c_void>>,
    },
}

pub struct MockScope<'s, 'p: 's> {
    inner: Arc<MockInner>,
    global: Arc<RefCell<HashMap<String, MockValue>>>,
    _marker: PhantomData<(&'s mut (), &'p ())>,
}

#[derive(Clone)]
pub struct MockGlobal {
    ctx: MockContext,
}

impl MockContext {
    #[must_use]
    pub fn new() -> Self {
        let global = Arc::new(RefCell::new(HashMap::new()));
        Self {
            inner: Arc::new(MockInner {
                registered: RefCell::new(HashMap::new()),
                global: global.clone(),
            }),
        }
    }

    pub fn with<R>(&self, f: impl for<'s> FnOnce(&mut MockScope<'s, 's>) -> Result<R, MockError>) -> Result<R, MockError> {
        let mut scope = MockScope {
            inner: self.inner.clone(),
            global: self.inner.global.clone(),
            _marker: PhantomData,
        };
        f(&mut scope)
    }
}

impl Default for MockContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime for MockRuntime {
    type Scope<'s, 'p: 's> = MockScope<'s, 'p>;
    type Value<'s> = MockValue;
    type Function<'s> = MockValue;
    type Persistent = MockGlobal;
    type Context = MockContext;
    type Error = MockError;

    fn name() -> &'static str {
        "mock"
    }

    fn version() -> String {
        "mock".to_string()
    }
}

impl ContextLike<MockRuntime> for MockContext {
    fn with_scope<T>(
        &self,
        f: impl for<'s> FnOnce(&mut MockScope<'s, 's>) -> Result<T, MockError>,
    ) -> Result<T, MockError> {
        self.with(f)
    }
}

impl ClassRegistry<MockRuntime> for MockContext {
    fn register_class<'s, T: NativeClass>(
        &self,
        scope: &mut MockScope<'s, 's>,
        descriptor: &'static crate::ClassDescriptor<MockRuntime>,
    ) -> Result<(), MockError> {
        let tid = TypeId::of::<T>();
        if self.inner.registered.borrow().contains_key(&tid) {
            return Ok(());
        }
        self.inner
            .registered
            .borrow_mut()
            .insert(tid, descriptor);

        let ctor = MockValue::Ctor(descriptor.constructor);

        let g = scope.global();
        g.set(scope, descriptor.name, ctor);
        Ok(())
    }

    fn wrap_native<'s, T: NativeClass>(
        scope: &mut <MockRuntime as Runtime>::Scope<'s, 's>,
        value: T,
    ) -> Result<<MockRuntime as Runtime>::Value<'s>, MockError> {
        let tid = TypeId::of::<T>();
        if !scope.inner.registered.borrow().contains_key(&tid) {
            return Err(MockError::new(format!("class {} not registered", T::NAME)));
        }
        let raw = Box::into_raw(Box::new(value));
        let ptr = Arc::new(Mutex::new(raw.cast()));
        Ok(MockValue::NativeObject {
            type_id: tid,
            ptr,
        })
    }

    fn unwrap_native<'s, T: NativeClass>(
        _scope: &mut <MockRuntime as Runtime>::Scope<'s, 's>,
        value: <MockRuntime as Runtime>::Value<'s>,
    ) -> Option<crate::NativeRef<'s, T>> {
        match value {
            MockValue::NativeObject { type_id, ptr } if type_id == TypeId::of::<T>() => {
                let guard = ptr.lock().ok()?;
                let p = (*guard).cast::<T>();
                if p.is_null() {
                    return None;
                }
                Some(unsafe { crate::NativeRef::new(p) })
            }
            _ => None,
        }
    }
}

impl<'js, 'p: 'js> ScopeLike<'js, 'p, MockRuntime> for MockScope<'js, 'p> {
    fn with_scope<'s2, F, T>(&'s2 mut self, f: F) -> T
    where
        'js: 's2,
        F: FnOnce(&mut MockScope<'s2, 'js>) -> T,
    {
        let mut inner = MockScope {
            inner: self.inner.clone(),
            global: self.global.clone(),
            _marker: PhantomData,
        };
        f(&mut inner)
    }

    fn eval(&mut self, _src: &str) -> Result<MockValue, MockError> {
        Err(MockError::new("MockRuntime::eval is not implemented"))
    }

    fn global(&mut self) -> MockValue {
        MockValue::Object(self.global.clone())
    }

    fn undefined(&mut self) -> MockValue {
        MockValue::Undefined
    }

    fn null(&mut self) -> MockValue {
        MockValue::Null
    }

    fn boolean(&mut self, value: bool) -> MockValue {
        MockValue::Bool(value)
    }

    fn integer(&mut self, value: i32) -> MockValue {
        MockValue::Number(value as f64)
    }

    fn number(&mut self, value: f64) -> MockValue {
        MockValue::Number(value)
    }

    fn string(&mut self, value: &str) -> MockValue {
        MockValue::String(value.to_string())
    }

    fn object(&mut self) -> MockValue {
        MockValue::Object(Arc::new(RefCell::new(HashMap::new())))
    }

    fn array(&mut self, len: u32) -> MockValue {
        let m = Arc::new(RefCell::new(HashMap::new()));
        for i in 0..len {
            m.borrow_mut()
                .insert(i.to_string(), MockValue::Undefined);
        }
        MockValue::Object(m)
    }

    fn array_buffer_copy(&mut self, bytes: &[u8]) -> MockValue {
        MockValue::String(String::from_utf8_lossy(bytes).into_owned())
    }

    fn try_catch<F>(&mut self, f: F) -> TryCatchResult<MockValue, MockError>
    where
        F: FnOnce(&mut MockScope<'js, 'p>) -> Result<MockValue, MockError>,
    {
        match f(self) {
            Ok(v) => TryCatchResult::Ok(v),
            Err(e) => TryCatchResult::Exception(e),
        }
    }

    fn array_buffer_zero_copy(&mut self, data: &'js [u8]) -> MockValue {
        self.array_buffer_copy(data)
    }

    fn function<F>(&mut self, _f: F) -> Result<MockValue, MockError>
    where
        F: for<'a> Fn(&mut MockScope<'a, 'a>, Args<'a, MockRuntime>) -> Result<MockValue, MockError>
            + Send
            + Sync
            + 'static,
    {
        Err(MockError::new("MockRuntime::function is not implemented"))
    }
}

impl<'js> ValueLike<'js, MockRuntime> for MockValue {
    fn is_undefined(&self) -> bool {
        matches!(self, MockValue::Undefined)
    }

    fn is_null(&self) -> bool {
        matches!(self, MockValue::Null)
    }

    fn is_boolean(&self) -> bool {
        matches!(self, MockValue::Bool(_))
    }

    fn is_number(&self) -> bool {
        matches!(self, MockValue::Number(_))
    }

    fn is_string(&self) -> bool {
        matches!(self, MockValue::String(_))
    }

    fn is_object(&self) -> bool {
        matches!(self, MockValue::Object(_) | MockValue::NativeObject { .. })
    }

    fn is_array(&self) -> bool {
        matches!(self, MockValue::Object(_))
    }

    fn is_function(&self) -> bool {
        matches!(self, MockValue::Function(_) | MockValue::Ctor(_))
    }

    fn is_array_buffer(&self) -> bool {
        false
    }

    fn as_bool(&self, _scope: &mut MockScope<'js, '_>) -> Option<bool> {
        match self {
            MockValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_i32(&self, _scope: &mut MockScope<'js, '_>) -> Option<i32> {
        match self {
            MockValue::Number(n) => Some(*n as i32),
            _ => None,
        }
    }

    fn as_f64(&self, _scope: &mut MockScope<'js, '_>) -> Option<f64> {
        match self {
            MockValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn with_str<F, T>(&self, _scope: &mut MockScope<'js, '_>, f: F) -> Option<T>
    where
        F: FnOnce(&str) -> T,
    {
        match self {
            MockValue::String(s) => Some(f(s)),
            _ => None,
        }
    }

    fn to_string_lossy(&self, _scope: &mut MockScope<'js, '_>) -> Option<String> {
        match self {
            MockValue::String(s) => Some(s.clone()),
            MockValue::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    fn get(&self, _scope: &mut MockScope<'js, '_>, key: &str) -> MockValue {
        match self {
            MockValue::Object(map) => map.borrow().get(key).cloned().unwrap_or(MockValue::Undefined),
            MockValue::Function(_) | MockValue::Ctor(_) => MockValue::Undefined,
            MockValue::NativeObject { .. } => MockValue::Undefined,
            _ => MockValue::Undefined,
        }
    }

    fn set(&self, _scope: &mut MockScope<'js, '_>, key: &str, val: MockValue) {
        if let MockValue::Object(map) = self {
            map.borrow_mut().insert(key.to_string(), val);
        }
    }

    fn has(&self, _scope: &mut MockScope<'js, '_>, key: &str) -> bool {
        match self {
            MockValue::Object(map) => map.borrow().contains_key(key),
            _ => false,
        }
    }

    fn delete(&self, _scope: &mut MockScope<'js, '_>, key: &str) -> bool {
        match self {
            MockValue::Object(map) => map.borrow_mut().remove(key).is_some(),
            _ => false,
        }
    }

    fn get_index(&self, scope: &mut MockScope<'js, '_>, i: u32) -> MockValue {
        self.get(scope, &i.to_string())
    }

    fn set_index(&self, scope: &mut MockScope<'js, '_>, i: u32, val: MockValue) {
        self.set(scope, &i.to_string(), val);
    }

    fn length(&self, _scope: &mut MockScope<'js, '_>) -> u32 {
        match self {
            MockValue::Object(map) => map.borrow().len() as u32,
            _ => 0,
        }
    }

    fn with_bytes<F, T>(&self, _scope: &mut MockScope<'js, '_>, _f: F) -> Option<T>
    where
        F: FnOnce(&[u8]) -> T,
    {
        None
    }

    fn call(
        &self,
        scope: &mut MockScope<'js, '_>,
        _this: MockValue,
        args: &[MockValue],
    ) -> Result<MockValue, MockError> {
        match self {
            MockValue::Function(f) => {
                let packed: Vec<_> = args.to_vec();
                let args = Args::new(MockValue::Undefined, packed);
                f(scope, args)
            }
            MockValue::Ctor(Some(cf)) => {
                let packed: Vec<_> = args.to_vec();
                let args = Args::new(MockValue::Undefined, packed);
                let scope_eq: &mut MockScope<'js, 'js> = unsafe { std::mem::transmute(scope) };
                cf(scope_eq, args)
            }
            MockValue::Ctor(None) => Err(HostError::type_error(E_TYPE, "not constructible").into()),
            _ => Err(HostError::type_error(E_TYPE, "not callable").into()),
        }
    }
}

impl<'a> JsFunction<'a, MockRuntime> for MockValue {}

impl PersistentLike<MockRuntime> for MockGlobal {
    fn new<'s, 'p: 's>(scope: &mut MockScope<'s, 'p>, value: MockValue) -> Self {
        let ctx = MockContext {
            inner: scope.inner.clone(),
        };
        ctx.with(move |s| {
            s.global().set(s, "__persistent", value);
            Ok(())
        })
        .expect("persistent save");
        Self { ctx }
    }

    fn get<'s, 'p: 's>(&self, _scope: &mut MockScope<'s, 'p>) -> MockValue {
        self.ctx
            .with(|s| Ok(s.global().get(s, "__persistent")))
            .unwrap_or(MockValue::Undefined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClassDescriptor, ClassRegistry, NativeRef};

    struct Point {
        x: f64,
        y: f64,
    }

    unsafe impl NativeClass for Point {
        const NAME: &'static str = "Point";

        fn descriptor<R: Runtime>() -> &'static ClassDescriptor<R> {
            crate::class_descriptor::<R, Point>(point_descriptor_owned::<R>)
        }
    }

    fn point_descriptor_owned<R: Runtime>() -> ClassDescriptor<R> {
        assert!(std::any::TypeId::of::<R>() == std::any::TypeId::of::<MockRuntime>());
        unsafe { std::mem::transmute_copy(point_descriptor_mock()) }
    }

    fn point_descriptor_mock() -> &'static ClassDescriptor<MockRuntime> {
        use once_cell::sync::OnceCell;
        static DESC: OnceCell<ClassDescriptor<MockRuntime>> = OnceCell::new();
        DESC.get_or_init(|| ClassDescriptor {
            name: "Point",
            constructor: Some(point_ctor as ConstructorFn<MockRuntime>),
            methods: &[],
            statics: &[],
            accessors: &[],
            symbols: &[],
            finalizer: point_fin,
        })
    }

    fn point_ctor<'a>(scope: &mut MockScope<'a, 'a>, args: Args<'a, MockRuntime>) -> Result<MockValue, MockError> {
        let x = args.get::<f64>(scope, 0)?;
        let y = args.get::<f64>(scope, 1)?;
        <MockContext as ClassRegistry<MockRuntime>>::wrap_native::<Point>(scope, Point { x, y })
    }

    unsafe fn point_fin(_p: *mut std::ffi::c_void) {}

    #[test]
    fn mock_class_wrap_unwrap() {
        let ctx = MockContext::new();
        ctx.with(|scope| {
            <MockContext as ClassRegistry<MockRuntime>>::register_class::<Point>(
                &ctx,
                scope,
                Point::descriptor::<MockRuntime>(),
            )
            .unwrap();
            let v = <MockContext as ClassRegistry<MockRuntime>>::wrap_native::<Point>(scope, Point { x: 1.0, y: 2.0 })
                .unwrap();
            let r: NativeRef<'_, Point> =
                <MockContext as ClassRegistry<MockRuntime>>::unwrap_native::<Point>(scope, v).unwrap();
            assert!((r.get().x - 1.0).abs() < 1e-9, "x={}", r.get().x);
            assert!((r.get().y - 2.0).abs() < 1e-9, "y={}", r.get().y);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn mock_unwrap_wrong_type_is_none() {
        struct Other;
        unsafe impl NativeClass for Other {
            const NAME: &'static str = "Other";
            fn descriptor<R: Runtime>() -> &'static ClassDescriptor<R> {
                unimplemented!()
            }
        }

        let ctx = MockContext::new();
        ctx.with(|scope| {
            <MockContext as ClassRegistry<MockRuntime>>::register_class::<Point>(
                &ctx,
                scope,
                Point::descriptor::<MockRuntime>(),
            )
            .unwrap();
            let v = <MockContext as ClassRegistry<MockRuntime>>::wrap_native::<Point>(scope, Point { x: 0.0, y: 0.0 })
                .unwrap();
            let o = <MockContext as ClassRegistry<MockRuntime>>::unwrap_native::<Other>(scope, v);
            assert!(o.is_none());
            Ok(())
        })
        .unwrap();
    }
}
