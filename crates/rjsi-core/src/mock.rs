use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;

use crate::{
    Context, Engine, ErasedNativeState, Error, MicrotaskDrainPolicy, NativeState, NativeStateSupport, Object, PropertyKey, Result, Runtime
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

pub struct MockContext<'js> {
    pub(crate) runtime: *mut MockRuntime,
    _marker: PhantomData<&'js mut ()>,
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
        pub struct $name<'js> {
            _p: PhantomData<&'js ()>,
        }

        impl<'js> $name<'js> {
            pub fn new() -> Self {
                Self { _p: PhantomData }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MockObject<'js> {
    pub(crate) id: u32,
    _p: PhantomData<&'js ()>,
}

impl<'js> MockObject<'js> {
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
pub struct MockValue<'js> {
    pub tag: u32,
    _p: PhantomData<&'js ()>,
}

impl<'js> MockValue<'js> {
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

pub struct MockRawArgs<'js> {
    pub argv: Vec<MockValue<'js>>,
    _p: PhantomData<&'js ()>,
}

impl<'js> MockRawArgs<'js> {
    pub fn from_slice(argv: &[MockValue<'js>]) -> Self {
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
    fn with_scope<R>(&mut self, f: impl for<'js> FnOnce(&mut Context<'js, MockEngine>) -> R) -> R {
        let cx_raw = MockContext {
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
    type Context<'js> = MockContext<'js>;
    type Value<'js> = MockValue<'js>;
    type Object<'js> = MockObject<'js>;
    type Function<'js> = MockFunction<'js>;
    type String<'js> = MockString<'js>;
    type Symbol<'js> = MockSymbol<'js>;
    type Key<'js> = MockKey<'js>;
    type PreparedKeyData = ();
    type RawArgs<'js> = MockRawArgs<'js>;
    type PersistentValue = MockPersistentValue;

    fn enter<'js>(runtime: &'js mut Self::Runtime) -> Self::Context<'js> {
        MockContext {
            runtime: runtime as *mut _,
            _marker: PhantomData,
        }
    }

    fn raw_args_len<'js>(args: &Self::RawArgs<'js>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'js>(args: &Self::RawArgs<'js>, index: usize) -> Option<Self::Value<'js>> {
        args.argv.get(index).copied()
    }

    fn eval<'js>(
        _cx: &mut Self::Context<'js>,
        _src: &str,
        _filename: Option<&str>,
    ) -> Result<Self::Value<'js>> {
        Ok(MockValue::UNDEFINED)
    }

    fn global_object<'js>(cx: &mut Self::Context<'js>) -> Self::Object<'js> {
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

    fn object_new<'js>(cx: &mut Self::Context<'js>) -> Result<Self::Object<'js>> {
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

    fn object_get<'js>(
        _cx: &mut Self::Context<'js>,
        _obj: &Self::Object<'js>,
        _key: PropertyKey<'js, Self>,
    ) -> Result<Self::Value<'js>> {
        Ok(MockValue::UNDEFINED)
    }

    fn object_set<'js>(
        _cx: &mut Self::Context<'js>,
        _obj: &Self::Object<'js>,
        _key: PropertyKey<'js, Self>,
        _val: Self::Value<'js>,
    ) -> Result<()> {
        Ok(())
    }

    fn object_has<'js>(
        _cx: &mut Self::Context<'js>,
        _obj: &Self::Object<'js>,
        _key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        Ok(false)
    }

    fn object_delete<'js>(
        _cx: &mut Self::Context<'js>,
        _obj: &Self::Object<'js>,
        _key: PropertyKey<'js, Self>,
    ) -> Result<bool> {
        Ok(true)
    }

    fn function_call<'js>(
        _cx: &mut Self::Context<'js>,
        _func: &Self::Function<'js>,
        _this: Self::Value<'js>,
        _args: &[Self::Value<'js>],
    ) -> Result<Self::Value<'js>> {
        Ok(MockValue::UNDEFINED)
    }

    fn value_is_undefined<'js>(val: &Self::Value<'js>) -> bool {
        val.tag == 0
    }
    fn value_is_null<'js>(val: &Self::Value<'js>) -> bool {
        val.tag == 1
    }
    fn value_is_boolean<'js>(val: &Self::Value<'js>) -> bool {
        val.tag == 2 || val.tag == 3
    }
    fn value_is_number<'js>(val: &Self::Value<'js>) -> bool {
        val.tag >= 4
    }
    fn value_is_string<'js>(_val: &Self::Value<'js>) -> bool {
        false
    }
    fn value_is_object<'js>(_val: &Self::Value<'js>) -> bool {
        false
    }
    fn value_is_function<'js>(_val: &Self::Value<'js>) -> bool {
        false
    }
    fn value_is_array<'js>(_val: &Self::Value<'js>) -> bool {
        false
    }
    fn value_is_symbol<'js>(_val: &Self::Value<'js>) -> bool {
        false
    }
    fn value_is_bigint<'js>(_val: &Self::Value<'js>) -> bool {
        false
    }

    fn make_undefined<'js>(_cx: &mut Self::Context<'js>) -> Self::Value<'js> {
        MockValue::UNDEFINED
    }
    fn make_null<'js>(_cx: &mut Self::Context<'js>) -> Self::Value<'js> {
        MockValue::NULL
    }
    fn make_bool<'js>(_cx: &mut Self::Context<'js>, v: bool) -> Self::Value<'js> {
        if v { MockValue::TRUE } else { MockValue::FALSE }
    }
    fn make_i32<'js>(_cx: &mut Self::Context<'js>, v: i32) -> Self::Value<'js> {
        MockValue::number(v.unsigned_abs())
    }
    fn make_f64<'js>(_cx: &mut Self::Context<'js>, v: f64) -> Self::Value<'js> {
        MockValue::number(v as u32)
    }
    fn make_string<'js>(_cx: &mut Self::Context<'js>, _s: &str) -> Result<Self::Value<'js>> {
        Ok(MockValue::UNDEFINED)
    }

    fn value_as_bool<'js>(val: &Self::Value<'js>) -> Option<bool> {
        match val.tag {
            2 => Some(false),
            3 => Some(true),
            _ => None,
        }
    }

    fn value_to_bool<'js>(_cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> bool {
        match val.tag {
            0 | 1 | 2 => false,
            3 => true,
            4 => false,
            _ => true,
        }
    }

    fn value_to_f64<'js>(_cx: &mut Self::Context<'js>, val: &Self::Value<'js>) -> Result<f64> {
        if val.tag >= 4 {
            Ok((val.tag - 4) as f64)
        } else {
            Err(Error::type_err("not a number"))
        }
    }

    fn value_to_string<'js>(
        _cx: &mut Self::Context<'js>,
        _val: &Self::Value<'js>,
    ) -> Result<String> {
        Ok(String::from("mock"))
    }

    fn object_to_value<'js>(_obj: Self::Object<'js>) -> Self::Value<'js> {
        MockValue::UNDEFINED
    }
    fn value_as_object<'js>(_val: Self::Value<'js>) -> Option<Self::Object<'js>> {
        None
    }
    fn function_to_value<'js>(_f: Self::Function<'js>) -> Self::Value<'js> {
        MockValue::UNDEFINED
    }
    fn value_as_function<'js>(_val: Self::Value<'js>) -> Option<Self::Function<'js>> {
        None
    }
    fn function_to_object<'js>(_f: Self::Function<'js>) -> Self::Object<'js> {
        MockObject::new()
    }

    fn persist_value<'js>(
        cx: &mut Self::Context<'js>,
        val: Self::Value<'js>,
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

    fn restore_value<'js>(
        cx: &mut Self::Context<'js>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'js>> {
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

    fn make_function<'js, F>(
        _cx: &mut Self::Context<'js>,
        _name: &str,
        _func: F,
    ) -> Result<Self::Function<'js>>
    where
        F: crate::args::RawHostFn<Self> + 'static,
    {
        todo!()
    }

    fn throw<'js>(_cx: &mut Self::Context<'js>, _value: Self::Value<'js>) -> Error {
        Error::Exception
    }
}

impl NativeStateSupport for MockEngine {
    fn object_create_with_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        state: S,
    ) -> Result<Object<'js, Self>> {
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

    fn object_get_state<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &Object<'js, Self>,
    ) -> Option<&'js S> {
        let mcx = crate::__cx::context_mut(cx);
        if mcx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &*mcx.runtime };
        let id = obj.as_raw().id;
        let slot = rt.native_states.get(&id)?;
        let r = slot.inner.downcast_ref::<S>()?;
        Some(unsafe { mem::transmute::<&S, &'js S>(r) })
    }

    fn object_get_state_mut<'js, S: NativeState>(
        cx: &mut Context<'js, Self>,
        obj: &mut Object<'js, Self>,
    ) -> Option<&'js mut S> {
        let mcx = crate::__cx::context_mut(cx);
        if mcx.runtime.is_null() {
            return None;
        }
        let rt = unsafe { &mut *mcx.runtime };
        let id = obj.as_raw().id;
        let slot = rt.native_states.get_mut(&id)?;
        let r = slot.inner.downcast_mut::<S>()?;
        Some(unsafe { mem::transmute::<&mut S, &'js mut S>(r) })
    }
}
