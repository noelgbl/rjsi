use std::marker::PhantomData;

use crate::{Context, Engine, Error, FromJs, PropertyKey, Result, ToJs};

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
    type PreparedKeyData = ();
    type RawArgs<'cx> = MockRawArgs<'cx>;

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        MockContext {
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

    fn global_object<'rt>(_cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        MockObject::new()
    }

    fn object_new<'rt>(_cx: &mut Self::Context<'rt>) -> Result<Self::Object<'rt>> {
        Ok(MockObject::new())
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

impl<'cx> ToJs<'cx, MockEngine> for u32 {
    fn to_js(self, _cx: &mut Context<'cx, MockEngine>) -> Result<MockValue<'cx>> {
        Ok(MockValue::number(self))
    }
}

impl<'cx> FromJs<'cx, MockEngine> for u32 {
    fn from_js(_cx: &mut Context<'cx, MockEngine>, value: MockValue<'cx>) -> Result<Self> {
        if value.tag >= 4 && (value.tag - 4) < 100 {
            Ok(value.tag - 4)
        } else {
            Err(Error::type_err("mock range"))
        }
    }
}
