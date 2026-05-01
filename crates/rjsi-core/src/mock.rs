use std::marker::PhantomData;

use crate::{
    Context, Engine, FromJs, InternKey, JsError, JsResult, Key, KeyCache, PropertyKey, StaticKeySlot, ToJs
};

pub struct MockEngine;

#[derive(Default)]
pub struct MockRuntime {
    pub atoms: Vec<String>,
    pub static_slots: Vec<Option<u32>>,
}

pub struct MockContext<'rt> {
    _marker: PhantomData<&'rt ()>,
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

phantom_val!(MockScope);
phantom_val!(MockObject);
phantom_val!(MockFunction);
phantom_val!(MockString);
phantom_val!(MockSymbol);
phantom_val!(MockKey);
phantom_val!(MockError);

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
            _marker: PhantomData,
        })
    }
}

impl Engine for MockEngine {
    type Runtime = MockRuntime;
    type Context<'rt> = MockContext<'rt>;
    type Scope<'cx> = MockScope<'cx>;
    type Value<'cx> = MockValue<'cx>;
    type Object<'cx> = MockObject<'cx>;
    type Function<'cx> = MockFunction<'cx>;
    type String<'cx> = MockString<'cx>;
    type Symbol<'cx> = MockSymbol<'cx>;
    type Key<'cx> = MockKey<'cx>;
    type Error<'cx> = MockError<'cx>;
    type RawArgs<'cx> = MockRawArgs<'cx>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        MockContext {
            _marker: PhantomData,
        }
    }

    fn raw_args_len<'cx>(args: &Self::RawArgs<'cx>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'cx>(args: &Self::RawArgs<'cx>, index: usize) -> Option<Self::Value<'cx>> {
        args.argv.get(index).copied()
    }

    fn eval<'cx>(
        _cx: &mut Self::Context<'_>,
        _src: &str,
        _filename: Option<&str>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        Ok(MockValue::UNDEFINED)
    }

    fn global_object<'cx>(_cx: &mut Self::Context<'_>) -> Self::Object<'cx> {
        MockObject::new()
    }

    fn object_new<'cx>(_cx: &mut Self::Context<'_>) -> JsResult<'cx, Self, Self::Object<'cx>> {
        Ok(MockObject::new())
    }

    fn object_get<'cx>(
        _cx: &mut Self::Context<'_>,
        _obj: &Self::Object<'cx>,
        _key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        Ok(MockValue::UNDEFINED)
    }

    fn object_set<'cx>(
        _cx: &mut Self::Context<'_>,
        _obj: &Self::Object<'cx>,
        _key: PropertyKey<'cx, Self>,
        _val: Self::Value<'cx>,
    ) -> JsResult<'cx, Self, ()> {
        Ok(())
    }

    fn object_has<'cx>(
        _cx: &mut Self::Context<'_>,
        _obj: &Self::Object<'cx>,
        _key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool> {
        Ok(false)
    }

    fn object_delete<'cx>(
        _cx: &mut Self::Context<'_>,
        _obj: &Self::Object<'cx>,
        _key: PropertyKey<'cx, Self>,
    ) -> JsResult<'cx, Self, bool> {
        Ok(true)
    }

    fn function_call<'cx>(
        _cx: &mut Self::Context<'_>,
        _func: &Self::Function<'cx>,
        _this: Self::Value<'cx>,
        _args: &[Self::Value<'cx>],
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        Ok(MockValue::UNDEFINED)
    }

    fn value_is_undefined<'cx>(val: &Self::Value<'cx>) -> bool {
        val.tag == 0
    }
    fn value_is_null<'cx>(val: &Self::Value<'cx>) -> bool {
        val.tag == 1
    }
    fn value_is_boolean<'cx>(val: &Self::Value<'cx>) -> bool {
        val.tag == 2 || val.tag == 3
    }
    fn value_is_number<'cx>(val: &Self::Value<'cx>) -> bool {
        val.tag >= 4
    }
    fn value_is_string<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }
    fn value_is_object<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }
    fn value_is_function<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }
    fn value_is_array<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }
    fn value_is_symbol<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }
    fn value_is_bigint<'cx>(_val: &Self::Value<'cx>) -> bool {
        false
    }

    fn make_undefined<'cx>(_cx: &mut Self::Context<'_>) -> Self::Value<'cx> {
        MockValue::UNDEFINED
    }
    fn make_null<'cx>(_cx: &mut Self::Context<'_>) -> Self::Value<'cx> {
        MockValue::NULL
    }
    fn make_bool<'cx>(_cx: &mut Self::Context<'_>, v: bool) -> Self::Value<'cx> {
        if v { MockValue::TRUE } else { MockValue::FALSE }
    }
    fn make_i32<'cx>(_cx: &mut Self::Context<'_>, v: i32) -> Self::Value<'cx> {
        MockValue::number(v.unsigned_abs())
    }
    fn make_f64<'cx>(_cx: &mut Self::Context<'_>, v: f64) -> Self::Value<'cx> {
        MockValue::number(v as u32)
    }
    fn make_string<'cx>(
        _cx: &mut Self::Context<'_>,
        _s: &str,
    ) -> JsResult<'cx, Self, Self::Value<'cx>> {
        Ok(MockValue::UNDEFINED)
    }

    fn value_to_bool<'cx>(val: &Self::Value<'cx>) -> Option<bool> {
        match val.tag {
            2 => Some(false),
            3 => Some(true),
            _ => None,
        }
    }

    fn value_to_f64<'cx>(
        _cx: &mut Self::Context<'_>,
        val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, f64> {
        if val.tag >= 4 {
            Ok((val.tag - 4) as f64)
        } else {
            Err(JsError::type_err("not a number"))
        }
    }

    fn value_to_string_utf8<'cx>(
        _cx: &mut Self::Context<'_>,
        _val: &Self::Value<'cx>,
    ) -> JsResult<'cx, Self, String> {
        Ok(String::from("mock"))
    }

    fn object_to_value<'cx>(_obj: Self::Object<'cx>) -> Self::Value<'cx> {
        MockValue::UNDEFINED
    }
    fn value_to_object<'cx>(_val: Self::Value<'cx>) -> Option<Self::Object<'cx>> {
        None
    }
    fn function_to_value<'cx>(_f: Self::Function<'cx>) -> Self::Value<'cx> {
        MockValue::UNDEFINED
    }
    fn value_to_function<'cx>(_val: Self::Value<'cx>) -> Option<Self::Function<'cx>> {
        None
    }
    fn function_to_object<'cx>(_f: Self::Function<'cx>) -> Self::Object<'cx> {
        MockObject::new()
    }
    
    fn make_function<'cx, F>(
        cx: &mut Self::Context<'_>,
        name: &str,
        func: F,
    ) -> JsResult<'cx, Self, Self::Function<'cx>>
    where
        F: crate::args::RawHostFn<Self> + 'static {
        todo!()
    }
}

impl<'cx> ToJs<'cx, MockEngine> for u32 {
    fn to_js(self, _cx: &mut Context<'_, MockEngine>) -> JsResult<'cx, MockEngine, MockValue<'cx>> {
        Ok(MockValue::number(self))
    }
}

impl InternKey<MockEngine> for MockRuntime {
    fn intern_str<'cx>(
        &mut self,
        _cx: &mut Context<'_, MockEngine>,
        s: &str,
    ) -> JsResult<'cx, MockEngine, Key<'cx, MockEngine>> {
        let _atom = self.atoms.len() as u32;
        self.atoms.push(s.to_string());
        Ok(Key::new(MockKey { _p: PhantomData }))
    }
}

impl KeyCache<MockEngine> for MockRuntime {
    fn get_or_intern<'cx>(
        &mut self,
        _cx: &mut Context<'_, MockEngine>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, MockEngine, Key<'cx, MockEngine>> {
        let idx = slot.0 as usize;
        if idx >= self.static_slots.len() {
            self.static_slots.resize(idx + 1, None);
        }
        if self.static_slots[idx].is_none() {
            let a = self.atoms.len() as u32;
            self.atoms.push(format!("static_{}", slot.0));
            self.static_slots[idx] = Some(a);
        }
        Ok(Key::new(MockKey { _p: PhantomData }))
    }
}

impl<'cx> FromJs<'cx, MockEngine> for u32 {
    fn from_js(
        _cx: &mut Context<'_, MockEngine>,
        value: MockValue<'cx>,
    ) -> JsResult<'cx, MockEngine, Self> {
        if value.tag >= 4 && (value.tag - 4) < 100 {
            Ok(value.tag - 4)
        } else {
            Err(JsError::TypeError("mock range"))
        }
    }
}
