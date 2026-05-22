use rjsi_core::{Engine, Error, PropertyKey, Result};
use rquickjs::{
    Atom, Coerced, Ctx, Error as QError, Function, Object, String as QString, Symbol as QSymbol, Value
};

pub struct QuickJsEngine;

pub struct QuickJsContext<'js> {
    pub(crate) qctx: Ctx<'js>,
    pub(crate) runtime: *mut crate::runtime::QuickJsRuntime,
}

impl<'js> QuickJsContext<'js> {
    pub(crate) fn clone_ctx(&self) -> Ctx<'js> {
        self.qctx.clone()
    }
}

pub struct QuickJsArgs<'js> {
    pub(crate) argv: Vec<Value<'js>>,
}

pub(crate) fn map_err<'rt, T>(_cx: &QuickJsContext<'rt>, res: rquickjs::Result<T>) -> Result<T> {
    match res {
        Ok(v) => Ok(v),
        Err(QError::Exception) => Err(Error::Exception),
        Err(e) => Err(Error::from_host(e)),
    }
}

impl Engine for QuickJsEngine {
    type Runtime = crate::runtime::QuickJsRuntime;
    type Context<'rt> = QuickJsContext<'rt>;
    type Value<'cx> = Value<'cx>;
    type Object<'cx> = Object<'cx>;
    type Function<'cx> = Function<'cx>;
    type String<'cx> = QString<'cx>;
    type Symbol<'cx> = QSymbol<'cx>;
    type Key<'cx> = Atom<'cx>;
    type PreparedKeyData = crate::runtime::QuickJsPreparedKeyData;
    type RawArgs<'cx> = QuickJsArgs<'cx>;
    type PersistentValue = rquickjs::Persistent<rquickjs::Value<'static>>;
    const ENGINE_NAME: &str = "QuickJS";

    fn enter<'rt>(_runtime: &'rt mut Self::Runtime) -> Self::Context<'rt> {
        unreachable!("Use Runtime::with_scope instead for QuickJS")
    }

    fn raw_args_len<'rt>(args: &Self::RawArgs<'rt>) -> usize {
        args.argv.len()
    }

    fn raw_args_get<'rt>(args: &Self::RawArgs<'rt>, index: usize) -> Option<Self::Value<'rt>> {
        args.argv.get(index).cloned()
    }

    fn eval<'rt>(
        cx: &mut Self::Context<'rt>,
        src: &str,
        _filename: Option<&str>,
    ) -> Result<Self::Value<'rt>> {
        let res: rquickjs::Result<Value<'_>> = cx.qctx.eval(src);
        map_err(cx, res)
    }

    fn global_object<'rt>(cx: &mut Self::Context<'rt>) -> Self::Object<'rt> {
        cx.qctx.globals()
    }

    fn object_new<'rt>(cx: &mut Self::Context<'rt>) -> Result<Self::Object<'rt>> {
        let res = Object::new(cx.clone_ctx());
        map_err(cx, res)
    }

    fn object_get<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<Self::Value<'rt>> {
        let res: rquickjs::Result<Value<'_>> = match key {
            PropertyKey::Str(s) => obj.get(s),
            PropertyKey::Prepared(k) => obj.get(crate::runtime::prepared_key(cx, &k)?),
            PropertyKey::Symbol(s) => obj.get(s),
            PropertyKey::Index(i) => obj.get(i),
        };
        map_err(cx, res)
    }

    fn object_set<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
        val: Self::Value<'rt>,
    ) -> Result<()> {
        let val_local: Value<'_> = val;
        let res: rquickjs::Result<()> = match key {
            PropertyKey::Str(s) => obj.set(s, val_local),
            PropertyKey::Prepared(k) => obj.set(crate::runtime::prepared_key(cx, &k)?, val_local),
            PropertyKey::Symbol(s) => obj.set(s, val_local),
            PropertyKey::Index(i) => obj.set(i, val_local),
        };
        map_err(cx, res)
    }

    fn object_has<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => obj.contains_key(s),
            PropertyKey::Prepared(k) => obj.contains_key(crate::runtime::prepared_key(cx, &k)?),
            PropertyKey::Symbol(s) => obj.contains_key(s),
            PropertyKey::Index(i) => obj.contains_key(i),
        };
        map_err(cx, res)
    }

    fn object_delete<'rt>(
        cx: &mut Self::Context<'rt>,
        obj: &Self::Object<'rt>,
        key: PropertyKey<'rt, Self>,
    ) -> Result<bool> {
        let res: rquickjs::Result<bool> = match key {
            PropertyKey::Str(s) => {
                let _ = obj.remove(s);
                Ok(true)
            }
            PropertyKey::Prepared(k) => {
                let _ = obj.remove(crate::runtime::prepared_key(cx, &k)?);
                Ok(true)
            }
            PropertyKey::Symbol(s) => {
                let _ = obj.remove(s);
                Ok(true)
            }
            PropertyKey::Index(i) => {
                let _ = obj.remove(i);
                Ok(true)
            }
        };
        map_err(cx, res)
    }

    fn function_call<'rt>(
        cx: &mut Self::Context<'rt>,
        func: &Self::Function<'rt>,
        this: Self::Value<'rt>,
        args: &[Self::Value<'rt>],
    ) -> Result<Self::Value<'rt>> {
        let func_local: Function<'_> = func.clone();
        let this_local: Value<'_> = this;
        let mut fargs = rquickjs::function::Args::new(cx.clone_ctx(), args.len());
        let _ = fargs.this(this_local);
        for a in args {
            fargs.push_arg(a.clone()).unwrap();
        }
        let res: rquickjs::Result<Value<'_>> = func_local.call_arg(fargs);
        map_err(cx, res)
    }

    fn value_is_undefined<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_undefined()
    }
    fn value_is_null<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_null()
    }
    fn value_is_boolean<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_bool()
    }
    fn value_is_number<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_number()
    }
    fn value_is_string<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_string()
    }
    fn value_is_object<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_object()
    }
    fn value_is_function<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_function()
    }
    fn value_is_array<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_array()
    }
    fn value_is_symbol<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_symbol()
    }
    fn value_is_bigint<'rt>(val: &Self::Value<'rt>) -> bool {
        val.is_big_int()
    }

    fn make_undefined<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::new_undefined(cx.clone_ctx())
    }
    fn make_null<'rt>(cx: &mut Self::Context<'rt>) -> Self::Value<'rt> {
        Value::new_null(cx.clone_ctx())
    }
    fn make_bool<'rt>(cx: &mut Self::Context<'rt>, v: bool) -> Self::Value<'rt> {
        Value::new_bool(cx.clone_ctx(), v)
    }
    fn make_i32<'rt>(cx: &mut Self::Context<'rt>, v: i32) -> Self::Value<'rt> {
        Value::new_int(cx.clone_ctx(), v)
    }
    fn make_f64<'rt>(cx: &mut Self::Context<'rt>, v: f64) -> Self::Value<'rt> {
        Value::new_float(cx.clone_ctx(), v)
    }

    fn make_string<'rt>(cx: &mut Self::Context<'rt>, s: &str) -> Result<Self::Value<'rt>> {
        let res = QString::from_str(cx.clone_ctx(), s).map(|s| s.into_value());
        map_err(cx, res)
    }

    fn value_as_bool<'rt>(val: &Self::Value<'rt>) -> Option<bool> {
        val.as_bool()
    }

    fn value_to_bool<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> bool {
        let res: rquickjs::Result<Coerced<bool>> = val.clone().get();
        map_err(cx, res.map(|c| *c)).unwrap_or(false)
    }

    fn value_to_f64<'rt>(cx: &mut Self::Context<'rt>, val: &Self::Value<'rt>) -> Result<f64> {
        let res: rquickjs::Result<f64> = val.clone().get();
        map_err(cx, res)
    }

    fn value_to_string<'rt>(
        cx: &mut Self::Context<'rt>,
        val: &Self::Value<'rt>,
    ) -> Result<std::string::String> {
        let s: rquickjs::Result<Coerced<std::string::String>> = val.clone().get();
        map_err(cx, s.map(|c| (*c).clone()))
    }

    fn object_to_value<'rt>(obj: Self::Object<'rt>) -> Self::Value<'rt> {
        obj.into_value()
    }
    fn value_as_object<'rt>(val: Self::Value<'rt>) -> Option<Self::Object<'rt>> {
        val.into_object()
    }
    fn function_to_value<'rt>(f: Self::Function<'rt>) -> Self::Value<'rt> {
        f.into_value()
    }
    fn value_as_function<'rt>(val: Self::Value<'rt>) -> Option<Self::Function<'rt>> {
        val.into_function()
    }
    fn function_to_object<'rt>(f: Self::Function<'rt>) -> Self::Object<'rt> {
        f.into_value().into_object().unwrap()
    }

    fn persist_value<'rt>(
        cx: &mut Self::Context<'rt>,
        val: Self::Value<'rt>,
    ) -> Self::PersistentValue {
        rquickjs::Persistent::save(&cx.qctx, val)
    }

    fn restore_value<'rt>(
        cx: &mut Self::Context<'rt>,
        persisted: &Self::PersistentValue,
    ) -> Result<Self::Value<'rt>> {
        persisted
            .clone()
            .restore(&cx.qctx)
            .map_err(Error::from_host)
    }

    fn catch_exception<'rt>(cx: &mut Self::Context<'rt>) -> Option<Self::Value<'rt>> {
        let val = cx.qctx.catch();
        if val.is_undefined() { None } else { Some(val) }
    }

    fn make_function<'rt, F>(
        cx: &mut Self::Context<'rt>,
        name: &str,
        func: F,
    ) -> Result<Self::Function<'rt>>
    where
        F: rjsi_core::RawHostFn<Self> + 'static,
    {
        fn call_host_fn<'js, F: rjsi_core::RawHostFn<QuickJsEngine> + 'static>(
            func_cell: &std::cell::RefCell<F>,
            runtime: *mut crate::runtime::QuickJsRuntime,
            ctx: rquickjs::Ctx<'js>,
            this: rquickjs::function::This<rquickjs::Value<'js>>,
            args: rquickjs::function::Rest<rquickjs::Value<'js>>,
        ) -> rquickjs::Result<rquickjs::Value<'js>> {
            let mut context = rjsi_core::Context::new(QuickJsContext {
                qctx: ctx.clone(),
                runtime,
            });
            let this_core = rjsi_core::Value::new(this.0);
            let args_core = rjsi_core::Args::new(QuickJsArgs { argv: args.0 });

            match func_cell
                .borrow_mut()
                .call(&mut context, this_core, args_core)
            {
                Ok(v) => Ok(v.into_raw()),
                Err(rjsi_core::Error::Exception) => Err(rquickjs::Error::Exception),
                Err(e) => {
                    let msg = e.to_string();
                    let err = rquickjs::Exception::from_message(ctx.clone(), &msg).unwrap();
                    ctx.throw(err.into_value());
                    Err(rquickjs::Error::Exception)
                }
            }
        }

        let func_cell = std::cell::RefCell::new(func);
        let runtime = cx.runtime;
        let qjs_func = rquickjs::Function::new(cx.clone_ctx(), move |ctx, this, args| {
            call_host_fn(&func_cell, runtime, ctx, this, args)
        });

        if let Ok(f) = &qjs_func {
            if let Some(obj) = f.as_object() {
                let _ = obj.set("name", name);
            }
        }

        map_err(cx, qjs_func)
    }
}

impl rjsi_core::capabilities::Promises for QuickJsEngine {
    fn promise_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::Object<'rt>)> {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        let (promise, resolve, reject) = qctx.promise().map_err(|e| Error::Host(Box::new(e)))?;
        let resolver_obj =
            rquickjs::Object::new(qctx.clone()).map_err(|e| Error::Host(Box::new(e)))?;
        resolver_obj
            .set("resolve", resolve)
            .map_err(|e| Error::Host(Box::new(e)))?;
        resolver_obj
            .set("reject", reject)
            .map_err(|e| Error::Host(Box::new(e)))?;
        Ok((promise.into_value().into_object().unwrap(), resolver_obj))
    }

    fn promise_resolve<'rt>(
        _cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()> {
        let resolve: rquickjs::Function = resolver
            .get("resolve")
            .map_err(|e| Error::Host(Box::new(e)))?;
        resolve
            .call::<_, ()>((value,))
            .map_err(|e| Error::Host(Box::new(e)))?;
        Ok(())
    }

    fn promise_reject<'rt>(
        _cx: &mut rjsi_core::Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()> {
        let reject: rquickjs::Function = resolver
            .get("reject")
            .map_err(|e| Error::Host(Box::new(e)))?;
        reject
            .call::<_, ()>((reason,))
            .map_err(|e| Error::Host(Box::new(e)))?;
        Ok(())
    }

    fn promise_state<'rt>(
        _cx: &mut rjsi_core::Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<rjsi_core::capabilities::PromiseState> {
        let p: rquickjs::Promise = promise
            .clone()
            .into_value()
            .into_promise()
            .ok_or_else(|| Error::type_err("promise_state: object is not a Promise"))?;
        Ok(match p.state() {
            rquickjs::promise::PromiseState::Pending => {
                rjsi_core::capabilities::PromiseState::Pending
            }
            rquickjs::promise::PromiseState::Resolved => {
                rjsi_core::capabilities::PromiseState::Resolved
            }
            rquickjs::promise::PromiseState::Rejected => {
                rjsi_core::capabilities::PromiseState::Rejected
            }
        })
    }

    fn promise_result<'rt>(
        _cx: &mut rjsi_core::Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<Option<std::result::Result<Self::Value<'rt>, Self::Value<'rt>>>> {
        let p: rquickjs::Promise = promise
            .clone()
            .into_value()
            .into_promise()
            .ok_or_else(|| Error::type_err("promise_result: object is not a Promise"))?;
        match p.state() {
            rquickjs::promise::PromiseState::Pending => Ok(None),
            rquickjs::promise::PromiseState::Resolved => {
                let value: rquickjs::Value = p
                    .result()
                    .ok_or_else(|| Error::type_err("promise resolved without a value"))?
                    .map_err(|e| Error::Host(Box::new(e)))?;
                Ok(Some(Ok(value)))
            }
            rquickjs::promise::PromiseState::Rejected => {
                let value: rquickjs::Value = p
                    .result()
                    .ok_or_else(|| Error::type_err("promise rejected without a value"))?
                    .unwrap_or_else(|_| {
                        let qctx = p.ctx().clone();
                        let caught = qctx.catch();
                        caught
                    });
                Ok(Some(Err(value)))
            }
        }
    }
}

impl rjsi_core::capabilities::Microtasks for QuickJsEngine {
    fn queue_microtask<'rt>(cx: &mut rjsi_core::Context<'rt, Self>, task: Self::Function<'rt>) {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        let promise: rquickjs::Object = qctx.globals().get("Promise").unwrap();
        let resolve: rquickjs::Function = promise.get("resolve").unwrap();
        let resolved: rquickjs::Object = resolve
            .call::<_, rquickjs::Object>((rquickjs::Value::new_undefined(qctx.clone()),))
            .unwrap();
        let then: rquickjs::Function = resolved.get("then").unwrap();
        then.call::<_, ()>((task,)).unwrap();
    }

    fn drain_microtasks<'rt>(cx: &mut rjsi_core::Context<'rt, Self>) {
        let qctx = &rjsi_core::__cx::context_mut(cx).qctx;
        while qctx.execute_pending_job() {}
    }
}

unsafe extern "C" fn qjs_buffer_free(
    _rt: *mut rquickjs_sys::JSRuntime,
    opaque: *mut std::ffi::c_void,
    _ptr: *mut std::ffi::c_void,
) {
    if !opaque.is_null() {
        drop(unsafe { Box::from_raw(opaque as *mut rjsi_core::capabilities::BufferOwner) });
    }
}

fn qjs_typed_array_kind(
    val: rquickjs_sys::JSValue,
) -> Option<rjsi_core::capabilities::TypedArrayKind> {
    use rjsi_core::capabilities::TypedArrayKind;
    let t = unsafe { rquickjs_sys::JS_GetTypedArrayType(val) };
    if t < 0 {
        return None;
    }
    Some(match t as u32 {
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C => TypedArrayKind::Uint8Clamped,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8 => TypedArrayKind::Int8,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8 => TypedArrayKind::Uint8,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16 => TypedArrayKind::Int16,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16 => TypedArrayKind::Uint16,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32 => TypedArrayKind::Int32,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32 => TypedArrayKind::Uint32,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32 => TypedArrayKind::Float32,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64 => TypedArrayKind::Float64,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64 => TypedArrayKind::BigInt64,
        rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64 => TypedArrayKind::BigUint64,
        _ => return None,
    })
}

fn typed_array_kind_to_qjs(
    k: rjsi_core::capabilities::TypedArrayKind,
) -> rquickjs_sys::JSTypedArrayEnum {
    use rjsi_core::capabilities::TypedArrayKind;
    match k {
        TypedArrayKind::Uint8Clamped => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C,
        TypedArrayKind::Int8 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8,
        TypedArrayKind::Uint8 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
        TypedArrayKind::Int16 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16,
        TypedArrayKind::Uint16 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16,
        TypedArrayKind::Int32 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32,
        TypedArrayKind::Uint32 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32,
        TypedArrayKind::Float32 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32,
        TypedArrayKind::Float64 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64,
        TypedArrayKind::BigInt64 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64,
        TypedArrayKind::BigUint64 => rquickjs_sys::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64,
    }
}

impl rjsi_core::capabilities::Buffers for QuickJsEngine {
    fn value_is_array_buffer<'cx>(val: &Self::Value<'cx>) -> bool {
        unsafe { rquickjs_sys::JS_IsArrayBuffer(val.as_raw()) }
    }

    fn value_typed_array_kind<'cx>(
        val: &Self::Value<'cx>,
    ) -> Option<rjsi_core::capabilities::TypedArrayKind> {
        qjs_typed_array_kind(val.as_raw())
    }

    unsafe fn array_buffer_adopt<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        ptr: *mut u8,
        len: usize,
        owner: rjsi_core::capabilities::BufferOwner,
    ) -> Result<Self::Object<'rt>> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let opaque = Box::into_raw(Box::new(owner)) as *mut std::ffi::c_void;
        let raw = unsafe {
            rquickjs_sys::JS_NewArrayBuffer(
                ctx_ptr,
                ptr,
                len as _,
                Some(qjs_buffer_free),
                opaque,
                false,
            )
        };
        let value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        value
            .into_object()
            .ok_or_else(|| Error::type_err("array_buffer_adopt: not an Object"))
    }

    fn array_buffer_alloc<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        len: usize,
    ) -> Result<Self::Object<'rt>> {
        let mut boxed: Box<[u8]> = vec![0u8; len].into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        let owner: rjsi_core::capabilities::BufferOwner = Box::new(boxed);
        unsafe {
            <Self as rjsi_core::capabilities::Buffers>::array_buffer_adopt(cx, ptr, len, owner)
        }
    }

    fn typed_array_new<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        kind: rjsi_core::capabilities::TypedArrayKind,
        buffer: Self::Object<'rt>,
        byte_offset: usize,
        length: usize,
    ) -> Result<Self::Object<'rt>> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let off_val = rquickjs::Value::new_float(ctx.clone(), byte_offset as f64);
        let len_val = rquickjs::Value::new_float(ctx.clone(), length as f64);
        let buf_val: rquickjs::Value<'rt> = buffer.into_value();
        let mut argv: [rquickjs_sys::JSValue; 3] =
            [buf_val.as_raw(), off_val.as_raw(), len_val.as_raw()];
        let raw = unsafe {
            rquickjs_sys::JS_NewTypedArray(
                ctx_ptr,
                argv.len() as _,
                argv.as_mut_ptr(),
                typed_array_kind_to_qjs(kind),
            )
        };
        let value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        if value.is_exception() {
            return Err(Error::Exception);
        }
        value
            .into_object()
            .ok_or_else(|| Error::type_err("typed_array_new: not an Object"))
    }

    fn array_buffer_byte_length<'cx>(
        _cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<usize> {
        let ctx_ptr = obj.ctx().as_raw().as_ptr();
        let mut size: rquickjs_sys::size_t = 0;
        let ptr =
            unsafe { rquickjs_sys::JS_GetArrayBuffer(ctx_ptr, &mut size, obj.as_value().as_raw()) };
        if ptr.is_null() {
            return Err(Error::type_err(
                "array_buffer_byte_length: not an ArrayBuffer",
            ));
        }
        Ok(size as usize)
    }

    fn typed_array_info<'cx>(
        cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<rjsi_core::capabilities::TypedArrayInfo> {
        let kind = qjs_typed_array_kind(obj.as_value().as_raw())
            .ok_or_else(|| Error::type_err("typed_array_info: not a TypedArray"))?;
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let mut off: rquickjs_sys::size_t = 0;
        let mut len: rquickjs_sys::size_t = 0;
        let mut stp: rquickjs_sys::size_t = 0;
        let raw = unsafe {
            rquickjs_sys::JS_GetTypedArrayBuffer(
                ctx_ptr,
                obj.as_value().as_raw(),
                &mut off,
                &mut len,
                &mut stp,
            )
        };
        let buf_value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        if buf_value.is_exception() {
            return Err(Error::Exception);
        }
        let elt_size = stp as usize;
        let byte_length = len as usize;
        let length = if elt_size == 0 {
            0
        } else {
            byte_length / elt_size
        };
        Ok(rjsi_core::capabilities::TypedArrayInfo {
            kind,
            byte_offset: off as usize,
            byte_length,
            length,
        })
    }

    fn typed_array_buffer<'rt>(
        cx: &mut rjsi_core::Context<'rt, Self>,
        obj: &Self::Object<'rt>,
    ) -> Result<Self::Object<'rt>> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let raw = unsafe {
            rquickjs_sys::JS_GetTypedArrayBuffer(
                ctx_ptr,
                obj.as_value().as_raw(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let value = unsafe { rquickjs::Value::from_raw(ctx, raw) };
        if value.is_exception() {
            return Err(Error::Exception);
        }
        value
            .into_object()
            .ok_or_else(|| Error::type_err("typed_array_buffer: not an Object"))
    }

    fn array_buffer_copy_to<'cx>(
        _cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()> {
        let ctx_ptr = obj.ctx().as_raw().as_ptr();
        let mut size: rquickjs_sys::size_t = 0;
        let src_ptr =
            unsafe { rquickjs_sys::JS_GetArrayBuffer(ctx_ptr, &mut size, obj.as_value().as_raw()) };
        if src_ptr.is_null() {
            return Err(Error::type_err("array_buffer_copy_to: not an ArrayBuffer"));
        }
        if dst.len() != size as usize {
            return Err(Error::type_err("array_buffer_copy_to: dst length mismatch"));
        }
        if dst.is_empty() {
            return Ok(());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(src_ptr, dst.as_mut_ptr(), dst.len());
        }
        Ok(())
    }

    fn typed_array_copy_to<'cx>(
        cx: &mut rjsi_core::Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()> {
        let ctx = rjsi_core::__cx::context_mut(cx).clone_ctx();
        let ctx_ptr = ctx.as_raw().as_ptr();
        let mut off: rquickjs_sys::size_t = 0;
        let mut len: rquickjs_sys::size_t = 0;
        let raw_buf = unsafe {
            rquickjs_sys::JS_GetTypedArrayBuffer(
                ctx_ptr,
                obj.as_value().as_raw(),
                &mut off,
                &mut len,
                std::ptr::null_mut(),
            )
        };
        let buf_value = unsafe { rquickjs::Value::from_raw(ctx.clone(), raw_buf) };
        if buf_value.is_exception() {
            return Err(Error::Exception);
        }
        if dst.len() != len as usize {
            return Err(Error::type_err("typed_array_copy_to: dst length mismatch"));
        }
        if dst.is_empty() {
            return Ok(());
        }
        let mut buf_size: rquickjs_sys::size_t = 0;
        let buf_ptr =
            unsafe { rquickjs_sys::JS_GetArrayBuffer(ctx_ptr, &mut buf_size, buf_value.as_raw()) };
        if buf_ptr.is_null() {
            return Err(Error::type_err("typed_array_copy_to: backing buffer null"));
        }
        let src = unsafe { buf_ptr.add(off as usize) };
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), dst.len());
        }
        Ok(())
    }
}
