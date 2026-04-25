use criterion::{Criterion, criterion_group, criterion_main};
use rjsi_core::{
    ClonedGlobal, FromJsValue, HostError, JsArrayOps, JsContext, JsContextImpl, JsEngine,
    JsErrorFactory, JsExceptionThrower, JsObject, JsObjectOps, JsResult, JsTypeOf, JsValue,
    JsValueImpl, JsValueMapper, JsValueType, PropertyAttributes, RjsiJSError, Source,
    ThrownValueHandle, ThrownValueStore,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
struct MockValue {
    ptr: *const RefCell<MockKind>,
}

#[derive(Clone)]
enum MockKind {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(HashMap<String, MockValue>),
    Array(Vec<MockValue>),
    Function(fn(&[MockValue]) -> MockValue),
    Exception,
}

impl MockValue {
    fn new(kind: MockKind) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(RefCell::new(kind))),
        }
    }

    fn undefined() -> Self {
        Self::new(MockKind::Undefined)
    }

    fn as_key(&self) -> String {
        match &*self.kind().borrow() {
            MockKind::String(s) => s.clone(),
            MockKind::Number(n) => (*n as u32).to_string(),
            _ => String::new(),
        }
    }

    fn kind(&self) -> &RefCell<MockKind> {
        // Benchmark values are intentionally leaked for stable raw handles.
        unsafe { &*self.ptr }
    }
}

impl PartialEq for MockValue {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl Eq for MockValue {}

impl Hash for MockValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
    }
}

struct MockContext {
    global: MockValue,
    thrown: RefCell<ThrownValueStore<MockValue>>,
}

impl MockContext {
    fn new() -> Self {
        Self {
            global: MockValue::new(MockKind::Object(HashMap::new())),
            thrown: RefCell::new(ThrownValueStore::new()),
        }
    }
}

struct MockEngine;

impl JsEngine for MockEngine {
    type RawContext<'js> = &'js MockContext;
    type Value = MockValue;
    type Context = MockContext;
    type Global = ClonedGlobal<Self>;

    fn name() -> &'static str {
        "mock"
    }

    fn version() -> String {
        "bench".to_string()
    }

    fn raw_context_from_ref<'js>(ctx: &'js Self::Context) -> Self::RawContext<'js> {
        ctx
    }

    fn context<'js>(raw: &Self::RawContext<'js>) -> &'js Self::Context {
        raw
    }
}

impl JsContextImpl for MockContext {
    type Engine = MockEngine;
    type RawContext = ();
    type Value = MockValue;

    fn as_raw(&self) -> &Self::RawContext {
        &()
    }

    fn eval(&self, _source: Source) -> Self::Value {
        MockValue::undefined()
    }

    fn global(&self) -> Self::Value {
        self.global.clone()
    }

    fn register_class<JC>(&self) -> Self::Value
    where
        JC: rjsi_core::JsClass<Self::Engine>,
    {
        MockValue::undefined()
    }

    fn call(
        &self,
        function: &Self::Value,
        _this: Self::Value,
        argv: &[Self::Value],
    ) -> Self::Value {
        match &*function.kind().borrow() {
            MockKind::Function(f) => f(argv),
            _ => MockValue::undefined(),
        }
    }

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value) {
        (
            MockValue::undefined(),
            MockValue::undefined(),
            MockValue::undefined(),
        )
    }

    fn compile_to_bytecode(&self, _source: Source) -> Result<Vec<u8>, RjsiJSError> {
        Err(HostError::new(rjsi_core::error::E_NOT_SUPPORTED, "bytecode").into())
    }

    fn run_bytecode(&self, _bytes: &[u8]) -> Self::Value {
        MockValue::undefined()
    }

    fn capture_thrown(&self, value: Self::Value) -> ThrownValueHandle {
        self.thrown.borrow_mut().insert(value)
    }

    fn resolve_thrown(&self, handle: ThrownValueHandle) -> Option<Self::Value> {
        self.thrown.borrow().get(handle)
    }

    fn take_thrown(&self, handle: ThrownValueHandle) -> Option<Self::Value> {
        self.thrown.borrow_mut().take(handle)
    }

    fn class_get(&self, _id: std::any::TypeId) -> Option<Self::Value> {
        None
    }

    fn class_insert(&self, _id: std::any::TypeId, _value: Self::Value) -> JsResult<()> {
        Ok(())
    }
}

impl JsErrorFactory for MockContext {
    fn new_error(&self, _name: &str, message: impl AsRef<str>, _code: Option<&str>) -> Self::Value {
        MockValue::new(MockKind::String(message.as_ref().to_string()))
    }
}

impl JsExceptionThrower for MockContext {
    fn throw(&self, value: Self::Value) -> Self::Value {
        let _ = value;
        MockValue::new(MockKind::Exception)
    }
}

impl JsValueImpl for MockValue {
    type RawValue = *const RefCell<MockKind>;
    type Context = MockContext;

    fn from_borrowed_raw(
        _ctx: <Self::Context as JsContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        Self { ptr: value }
    }

    fn from_owned_raw(
        _ctx: <Self::Context as JsContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        Self { ptr: value }
    }

    fn into_raw_value(self) -> Self::RawValue {
        self.ptr
    }

    fn as_raw_value(&self) -> &Self::RawValue {
        &self.ptr
    }

    fn as_raw_context(&self) -> &<Self::Context as JsContextImpl>::RawContext {
        &()
    }

    fn create_null(_ctx: &Self::Context) -> Self {
        Self::new(MockKind::Null)
    }

    fn create_undefined(_ctx: &Self::Context) -> Self {
        Self::undefined()
    }

    fn create_symbol(_ctx: &Self::Context, description: &str) -> Self {
        Self::new(MockKind::String(description.to_string()))
    }

    fn from_json_str(_ctx: &Self::Context, _str: &str) -> Self {
        Self::new(MockKind::Object(HashMap::new()))
    }

    fn create_date(_ctx: &Self::Context, epoch_ms: f64) -> Self {
        Self::new(MockKind::Number(epoch_ms))
    }
}

impl JsTypeOf for MockValue {
    fn value_type(&self) -> JsValueType {
        match &*self.kind().borrow() {
            MockKind::Undefined => JsValueType::Undefined,
            MockKind::Null => JsValueType::Null,
            MockKind::Bool(_) => JsValueType::Boolean,
            MockKind::Number(_) => JsValueType::Number,
            MockKind::String(_) => JsValueType::String,
            MockKind::Object(_) => JsValueType::Object,
            MockKind::Array(_) => JsValueType::Array,
            MockKind::Function(_) => JsValueType::Function,
            MockKind::Exception => JsValueType::Exception,
        }
    }

    fn is_exception(&self) -> bool {
        matches!(self.value_type(), JsValueType::Exception)
    }
    fn is_error(&self) -> bool {
        false
    }
    fn is_array(&self) -> bool {
        matches!(self.value_type(), JsValueType::Array)
    }
    fn is_array_buffer(&self) -> bool {
        false
    }
    fn is_promise(&self) -> bool {
        false
    }
    fn is_undefined(&self) -> bool {
        matches!(self.value_type(), JsValueType::Undefined)
    }
    fn is_null(&self) -> bool {
        matches!(self.value_type(), JsValueType::Null)
    }
    fn is_boolean(&self) -> bool {
        matches!(self.value_type(), JsValueType::Boolean)
    }
    fn is_number(&self) -> bool {
        matches!(self.value_type(), JsValueType::Number)
    }
    fn is_bigint(&self) -> bool {
        false
    }
    fn is_string(&self) -> bool {
        matches!(self.value_type(), JsValueType::String)
    }
    fn is_symbol(&self) -> bool {
        false
    }
    fn is_function(&self) -> bool {
        matches!(self.value_type(), JsValueType::Function)
    }
    fn is_object(&self) -> bool {
        matches!(
            self.value_type(),
            JsValueType::Object | JsValueType::Array | JsValueType::Function
        )
    }
    fn is_constructor(&self) -> bool {
        false
    }
    fn is_date(&self) -> bool {
        false
    }
    fn is_proxy(&self) -> bool {
        false
    }
}

impl JsObjectOps for MockValue {
    fn new_object(_ctx: &Self::Context) -> Self {
        Self::new(MockKind::Object(HashMap::new()))
    }

    fn make_instance(_ctx: &Self::Context, _constructor: Self, _data: *mut ()) -> Self {
        Self::new(MockKind::Object(HashMap::new()))
    }

    fn instance_of(&self, _constructor: Self) -> bool {
        false
    }

    fn get_opaque(&self) -> *mut () {
        std::ptr::null_mut()
    }

    fn del_property(&self, key: Self) -> Result<bool, Self> {
        let removed = match &mut *self.kind().borrow_mut() {
            MockKind::Object(props) => props.remove(&key.as_key()).is_some(),
            _ => false,
        };
        Ok(removed)
    }

    fn has_property(&self, key: Self) -> Result<bool, Self> {
        let key = key.as_key();
        Ok(match &*self.kind().borrow() {
            MockKind::Object(props) => props.contains_key(&key),
            MockKind::Array(_) => key == "length",
            _ => false,
        })
    }

    fn set_property(&self, key: Self, value: Self) -> Result<(), Self> {
        match &mut *self.kind().borrow_mut() {
            MockKind::Object(props) => {
                props.insert(key.as_key(), value);
            }
            MockKind::Array(values) if key.as_key() == "length" => {
                if let MockKind::Number(length) = &*value.kind().borrow() {
                    values.resize_with(*length as usize, MockValue::undefined);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn set_prototype(&self, _prototype: Self) -> bool {
        true
    }

    fn define_property(
        &self,
        key: Self,
        value: Self,
        _getter: Self,
        _setter: Self,
        _attributes: PropertyAttributes,
    ) -> Result<(), Self> {
        self.set_property(key, value)
    }

    fn get_property(&self, key: Self) -> Result<Option<Self>, Self> {
        let key = key.as_key();
        Ok(match &*self.kind().borrow() {
            MockKind::Object(props) => props.get(&key).cloned(),
            MockKind::Array(values) if key == "length" => {
                Some(MockValue::new(MockKind::Number(values.len() as f64)))
            }
            _ => None,
        })
    }

    fn get_own_property_names(&self) -> Result<Vec<Self>, Self> {
        Ok(match &*self.kind().borrow() {
            MockKind::Object(props) => props
                .keys()
                .map(|key| MockValue::new(MockKind::String(key.clone())))
                .collect(),
            _ => Vec::new(),
        })
    }
}

impl JsArrayOps for MockValue {
    fn new_array(_ctx: &Self::Context) -> Self {
        Self::new(MockKind::Array(Vec::new()))
    }

    fn get_index(&self, index: u32) -> Self {
        match &*self.kind().borrow() {
            MockKind::Array(values) => values
                .get(index as usize)
                .cloned()
                .unwrap_or_else(MockValue::undefined),
            _ => MockValue::undefined(),
        }
    }

    fn set_index(&self, index: u32, value: Self) -> Self {
        if let MockKind::Array(values) = &mut *self.kind().borrow_mut() {
            let index = index as usize;
            if values.len() <= index {
                values.resize_with(index + 1, MockValue::undefined);
            }
            values[index] = value;
        }
        MockValue::undefined()
    }
}

impl From<(&MockContext, bool)> for MockValue {
    fn from((_, value): (&MockContext, bool)) -> Self {
        Self::new(MockKind::Bool(value))
    }
}

macro_rules! number_from {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<(&MockContext, $ty)> for MockValue {
                fn from((_, value): (&MockContext, $ty)) -> Self {
                    Self::new(MockKind::Number(value as f64))
                }
            }
        )*
    };
}

number_from!(i32, u32, i64, u64, f64);

impl From<(&MockContext, &str)> for MockValue {
    fn from((_, value): (&MockContext, &str)) -> Self {
        Self::new(MockKind::String(value.to_string()))
    }
}

impl TryInto<bool> for MockValue {
    type Error = RjsiJSError;

    fn try_into(self) -> Result<bool, Self::Error> {
        match &*self.kind().borrow() {
            MockKind::Bool(value) => Ok(*value),
            _ => Err(HostError::new(rjsi_core::error::E_TYPE, "not bool").into()),
        }
    }
}

macro_rules! number_try_into {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryInto<$ty> for MockValue {
                type Error = RjsiJSError;

                fn try_into(self) -> Result<$ty, Self::Error> {
                    match &*self.kind().borrow() {
                        MockKind::Number(value) => Ok(*value as $ty),
                        _ => Err(HostError::new(rjsi_core::error::E_TYPE, "not number").into()),
                    }
                }
            }
        )*
    };
}

number_try_into!(i32, u32, i64, u64, f64);

impl TryInto<String> for MockValue {
    type Error = RjsiJSError;

    fn try_into(self) -> Result<String, Self::Error> {
        match &*self.kind().borrow() {
            MockKind::String(value) => Ok(value.clone()),
            _ => Err(HostError::new(rjsi_core::error::E_TYPE, "not string").into()),
        }
    }
}

fn context() -> MockContext {
    MockContext::new()
}

fn js_context(ctx: &MockContext) -> JsContext<'_, MockEngine> {
    JsContext::new(ctx)
}

fn bench_values(c: &mut Criterion) {
    c.bench_function("value_from_rust_i32", |b| {
        let ctx = context();
        b.iter(|| JsValue::<MockEngine>::from_rust(js_context(&ctx), 42_i32));
    });

    c.bench_function("value_to_rust_i32", |b| {
        let ctx = context();
        let value = JsValue::<MockEngine>::from_rust(js_context(&ctx), 42_i32);
        b.iter(|| value.clone().to_rust::<i32>().unwrap());
    });
}

fn bench_object(c: &mut Criterion) {
    c.bench_function("object_get_required", |b| {
        let ctx = context();
        let obj = JsObject::<MockEngine>::new(js_context(&ctx));
        obj.set("answer", 42_i32).unwrap();
        b.iter(|| obj.get::<_, i32>("answer").unwrap());
    });

    c.bench_function("object_get_raw", |b| {
        let ctx = context();
        let obj = JsObject::<MockEngine>::new(js_context(&ctx));
        obj.set("answer", 42_i32).unwrap();
        b.iter(|| obj.get_raw("answer").unwrap().unwrap());
    });
}

fn bench_call(c: &mut Criterion) {
    fn returns_len(argv: &[MockValue]) -> MockValue {
        MockValue::new(MockKind::Number(argv.len() as f64))
    }

    c.bench_function("function_call_argv", |b| {
        let ctx = context();
        let raw = MockValue::new(MockKind::Function(returns_len));
        let func = rjsi_core::JsFunc::<MockEngine>::from_js_value(
            js_context(&ctx),
            JsValue::from_raw(js_context(&ctx), raw),
        )
        .unwrap();
        let argv = [MockValue::new(MockKind::Number(1.0))];
        b.iter(|| func.call_argv::<i32>(None, &argv).unwrap());
    });
}

fn bench_array(c: &mut Criterion) {
    c.bench_function("array_extend_100", |b| {
        let ctx = context();
        b.iter(|| {
            let array = rjsi_core::JsArray::<MockEngine>::new(js_context(&ctx)).unwrap();
            array.extend::<_, _>(0..100_i32).unwrap()
        });
    });
}

fn bench_exception(c: &mut Criterion) {
    c.bench_function("exception_try_convert", |b| {
        let ctx = context();
        let thrown = MockValue::new(MockKind::Exception);
        b.iter(|| {
            let result: Result<JsValue<'_, MockEngine>, _> =
                thrown.clone().try_convert(js_context(&ctx));
            result.err().unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_values,
    bench_object,
    bench_call,
    bench_array,
    bench_exception
);
criterion_main!(benches);
