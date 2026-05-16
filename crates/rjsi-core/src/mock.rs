use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;

use crate::{
    Context, Engine, ErasedNativeState, Error, FromJs, MicrotaskDrainPolicy, NativeState, NativeStateSupport, Object, PropertyKey, Result, Runtime, ToJs, Value
};

pub struct MockEngine;

#[derive(Default)]
pub struct MockRuntime {
    pub atoms: Vec<String>,
    pub static_slots: Vec<Option<u32>>,
    pub(crate) persistent_slots: Vec<Option<u32>>,
    pub(crate) next_object_id: u32,
    pub(crate) native_states: HashMap<u32, ErasedNativeState>,
}

impl MockRuntime {
    pub(crate) fn alloc_object_id(&mut self) -> u32 {
        self.next_object_id = self.next_object_id.wrapping_add(1);
        if self.next_object_id == 0 {
            self.next_object_id = 1;
        }
        self.next_object_id
    }
}

pub struct MockContext<'rt> {
    pub(crate) runtime: *mut MockRuntime,
    _marker: PhantomData<&'rt mut ()>,
}

pub struct MockPersistentValue {
    pub(crate) runtime: *mut MockRuntime,
    pub(crate) slot: usize,
}

impl Drop for MockPersistentValue {
    fn drop(&mut self) {
        unsafe {
            if self.runtime.is_null() {
                return;
            }
            let rt = &mut *self.runtime;
            if let Some(slot) = rt.persistent_slots.get_mut(self.slot) {
                *slot = None;
            }
        }
    }
}

macro_rules! phantom_val {
    ($name:ident) => {
        #[derive(Clone, Copy)]
        pub struct $name<'cx> {
            _p: PhantomData<&'cx ()>,
        }

        impl<'cx> $name<'cx> {
            pub fn new() -> Self {
                Self { _p: PhantomData }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MockObject<'cx> {
    pub(crate) id: u32,
    _p: PhantomData<&'cx ()>,
}

impl<'cx> MockObject<'cx> {
    pub fn new() -> Self {
        Self {
            id: 0,
            _p: PhantomData,
        }
    }
}
phantom_val!(MockFunction);
phantom_val!(MockString);
phantom_val!(MockSymbol);
phantom_val!(MockKey);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MockValue<'cx> {
    pub tag: u32,
    _p: PhantomData<&'cx ()>,
}

impl<'cx> MockValue<'cx> {
    pub const UNDEFINED: Self = Self {
        tag: 0,
        _p: PhantomData,
    };
    pub const NULL: Self = Self {
        tag: 1,
        _p: PhantomData,
    };
    pub const FALSE: Self = Self {
        tag: 2,
        _p: PhantomData,
    };
    pub const TRUE: Self = Self {
        tag: 3,
        _p: PhantomData,
    };
    pub fn number(v: u32) -> Self {
        Self {
            tag: 4 + v,
            _p: PhantomData,
        }
    }
}

pub struct MockRawArgs<'cx> {
    pub argv: Vec<MockValue<'cx>>,
    _p: PhantomData<&'cx ()>,
}

impl<'cx> MockRawArgs<'cx> {
    pub fn from_slice(argv: &[MockValue<'cx>]) -> Self {
        Self {
            argv: argv.to_vec(),
            _p: PhantomData,
        }
    }
}

impl MockEngine {
    pub fn detached_cx() -> Context<'static, MockEngine> {
        Context::new(MockContext {
            runtime: std::ptr::null_mut(),
            _marker: PhantomData,
        })
    }
}

impl Runtime<MockEngine> for MockRuntime {
    fn with_scope<R>(&mut self, f: impl for<'rt> FnOnce(&mut Context<'rt, MockEngine>) -> R) -> R {
        let mut cx_raw = MockContext {
            runtime: self as *mut _,
            _marker: PhantomData,
        };
        let mut cx = Context::new(cx_raw);
        f(&mut cx)
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        MicrotaskDrainPolicy::Explicit
    }

    fn set_microtask_policy(&mut self, _policy: MicrotaskDrainPolicy) {}
}

impl Engine for MockEngine {
    const ENGINE_NAME: &str = "Mock";

    type Runtime = MockRuntime;
    type Context<'rt> = MockContext<'rt>;
    type Value<'cx> = MockValue<'cx>;
    type Object<'cx> = MockObject<'cx>;
    type Function<'cx> = MockFunction<'cx>;
    type String<'cx> = MockString<'cx>;
    type Symbol<'cx> = MockSymbol<'cx>;
    type Key<'cx> = MockKey<'cx>;
    type PreparedKeyData = ();
    type RawArgs<'cx> = MockRawArgs<'cx>;
    type PersistentValue = MockPersistentValue;

    fn enter<'rt>(runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        MockContext {
            runtime: runtime as *mut _,
            _marker: PhantomData,
        }
    }

    fn raw_args_len<'rt>(args: &Self::RawArgs<'rt>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'rt>(args: &Self::RawArgs<'rt>, index: usize) -> Option<Self::Value<'rt>> {
        args.argv.get(index).copied()
    }

    fn eval<'rt>(
        _cx: &mut Self::Context<'rt>,
        _src: &str,
        _filename: Option<&str>,
    ) -> Result<Self::Value<'rt>> {
        Ok(MockValue::UNDEFINED)
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        if cx.runtime.is_null() {
            return MockObject::new();
        }
        let rt = unsafe { &mut *cx.runtime };
        let id = rt.alloc_object_id();
        MockObject {
            id,
            _p: PhantomData,
        }
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> Result<Self::Object<'rt>> {
        if cx.runtime.is_null() {
            return Ok(MockObject::new());
        }
        let rt = unsafe { &mut *cx.runtime };
        let id = rt.alloc_object_id();
        Ok(MockObject {
            id,
            _p: PhantomData,
        })
    }

    fn object_get<'rt>(
        _cx: &mut Self::Context<'rt>,
        _obj: &Self::Object<'rt>,
        _key: PropertyKey<'rt, Self>,
    ) -> Result<Self::Value<'rt>> {
        Ok(MockValue::UNDEFINED)
    }

    fn object_set<'rt>(
        _cx: &mut Self::Context<'rt>,
        _obj: &Self::Object<'rt>,
        _key: PropertyKey<'rt, Self>,
        _val: Self::Value<'rt>,
    ) -> Result<()> {
        Ok(())
    }

    fn object_has<'rt>(
        _cx: &mut Self::Context<'rt>,
        _obj: &Self::Object<'rt>,
        _key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn object_delete<'rt>(
        _cx: &mut Self::Context<'rt>,
        _obj: &Self::Object<'rt>,
        _key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        Ok(true)
    }

    fn function_call<'rt>(
        _cx: &mut Self::Context<'rt>,
        _func: &Self::Function<'rt>,
        _this: Self::Value<'rt>,
        _args: &[Self::Value<'rt>],
    ) -> Result<Self::Value<'rt>> {
        Ok(MockValue::UNDEFINED)
    }

    fn value_is_undefined<'rt>(val: &Self::Value<'rt>) -> bool {
        val.tag == 0
    }
    fn value_is_null<'rt>(val: &Self::Value<'rt>) -> bool {
        val.tag == 1
    }
    fn value_is_boolean<'rt>(val: &Self::Value<'rt>) -> bool {
        val.tag == 2 || val.tag == 3
    }
    fn value_is_number<'rt>(val: &Self::Value<'rt>) -> bool {
        val.tag >= 4
    }
    fn value_is_string<'rt>(_val: &Self::Value<'rt>) -> bool {
        false
    }
    fn value_is_object<'rt>(_val: &Self::Value<'rt>) -> bool {
        false
    }
    fn value_is_function<'rt>(_val: &Self::Value<'rt>) -> bool {
        false
    }
    fn value_is_array<'rt>(_val: &Self::Value<'rt>) -> bool {
        false
    }
    fn value_is_symbol<'rt>(_val: &Self::Value<'rt>) -> bool {
        false
    }
    fn value_is_bigint<'rt>(_val: &Self::Value<'rt>) -> bool {
        false
    }

    fn make_undefined<'rt>(_cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        MockValue::UNDEFINED
    }
    fn make_null<'rt>(_cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        MockValue::NULL
    }
    fn make_bool<'rt>(_cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        if v { MockValue::TRUE } else { MockValue::FALSE }
    }
    fn make_i32<'rt>(_cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        MockValue::number(v.unsigned_abs())
    }
    fn make_f64<'rt>(_cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        MockValue::number(v as u32)
    }
    fn make_string<'rt>(_cx: &mut Self::Context<'rt>, _s: &str) -> Result<Self::Value<'rt>> {
        Ok(MockValue::UNDEFINED)
    }

    fn value_to_bool<'rt>(val: &Self::Value<'rt>) -> Option<bool> {
        match val.tag {
            2 => Some(false),
            3 => Some(true),
            _ => None,
        }
    }

    fn value_to_f64<'rt>(_cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> Result<f64> {
        if val.tag >= 4 {
            Ok((val.tag - 4) as f64)
        } else {
            Err(Error::type_err("not a number"))
        }
    }

    fn value_to_string_utf8<'rt>(
        _cx: &mut Self::Context<'rt>,
        _val: &Self::Value<'rt>,
    ) -> Result<String> {
        Ok(String::from("mock"))
    }

    fn object_to_value<'rt>(_obj: Self::Object<'rt>) -> Self::Value<'rt> {
        MockValue::UNDEFINED
    }
    fn value_to_object<'rt>(_val: Self::Value<'rt>) -> Option<Self::Object<'rt>> {
        None
    }
    fn function_to_value<'rt>(_f: Self::Function<'rt>) -> Self::Value<'rt> {
        MockValue::UNDEFINED
    }
    fn value_to_function<'rt>(_val: Self::Value<'rt>) -> Option<Self::Function<'rt>> {
        None
    }
    fn function_to_object<'rt>(_f: Self::Function<'rt>) -> Self::Object<'rt> {
        MockObject::new()
    }

    fn persist_value<'rt>(
        cx: &mut Self::Context<'rt>,
        val: Self::Value<'rt>,
    ) -> Self::PersistentValue {
        assert!(
            !cx.runtime.is_null(),
            "MockEngine::persist_value requires a MockRuntime-backed Context (use Runtime::with_scope)"
        );
        let rt = unsafe { &mut *cx.runtime };
        let slot = if let Some(i) = rt.persistent_slots.iter().position(|s| s.is_none()) {
            i
        } else {
            rt.persistent_slots.push(None);
            rt.persistent_slots.len() - 1
        };
        rt.persistent_slots[slot] = Some(val.tag);
        MockPersistentValue {
            runtime: cx.runtime,
            slot,
        }
    }

    fn restore_value<'rt>(
        cx: &mut Self::Context<'rt>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'rt>> {
        assert!(
            !cx.runtime.is_null(),
            "MockEngine::restore_value requires a MockRuntime-backed Context (use Runtime::with_scope)"
        );
        let rt = unsafe { &*cx.runtime };
        let tag = rt
            .persistent_slots
            .get(persisted.slot)
            .copied()
            .flatten()
            .ok_or_else(|| Error::type_err("persistent slot empty"))?;
        Ok(MockValue {
            tag,
            _p: PhantomData,
        })
    }

    fn make_function<'rt, F>(
        _cx: &mut Self::Context<'rt>,
        _name: &str,
        _func: F,
    ) -> Result<Self::Function<'rt>>
    where
        F: crate::args::RawHostFn<Self> + 'static,
    {
        todo!()
    }
}

impl NativeStateSupport for MockEngine {
    fn object_create_with_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        state: S,
    ) -> Result<Object<'cx, Self>> {
        let mcx = crate::__cx::context_mut(cx);
        if mcx.runtime.is_null() {
            return Err(Error::type_err(
                "MockEngine::object_create_with_state requires MockRuntime-backed Context",
            ));
        }
        let rt = unsafe { &mut *mcx.runtime };
        let id = rt.alloc_object_id();
        rt.native_states.insert(
            id,
            ErasedNativeState {
                inner: Box::new(state),
            },
        );
        Ok(Object::new(MockObject {
            id,
            _p: PhantomData,
        }))
    }

    fn object_get_state<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &Object<'cx, Self>,
    ) -> Option<&'cx S> {
        let mcx = crate::__cx::context_mut(cx);
        if mcx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &*mcx.runtime };
        let id = obj.as_raw().id;
        let slot = rt.native_states.get(&id)?;
        let r = slot.inner.downcast_ref::<S>()?;
        Some(unsafe { mem::transmute::<&S, &'cx S>(r) })
    }

    fn object_get_state_mut<'cx, S: NativeState>(
        cx: &mut Context<'cx, Self>,
        obj: &mut Object<'cx, Self>,
    ) -> Option<&'cx mut S> {
        let mcx = crate::__cx::context_mut(cx);
        if mcx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &mut *mcx.runtime };
        let id = obj.as_raw().id;
        let slot = rt.native_states.get_mut(&id)?;
        let r = slot.inner.downcast_mut::<S>()?;
        Some(unsafe { mem::transmute::<&mut S, &'cx mut S>(r) })
    }
}

impl<'cx> ToJs<'cx, MockEngine> for u32 {
    fn to_js(self, _cx: &mut Context<'cx, MockEngine>) -> Result<Value<'cx, MockEngine>> {
        Ok(Value::new(MockValue::number(self)))
    }
}

impl<'cx> FromJs<'cx, MockEngine> for u32 {
    fn from_js(_cx: &mut Context<'cx, MockEngine>, value: Value<'cx, MockEngine>) -> Result<Self> {
        if value.is_number() && (value.to_f64(_cx)? as u32) < 100 {
            Ok(value.to_f64(_cx)? as u32)
        } else {
            Err(Error::type_err("mock range"))
        }
    }
}
