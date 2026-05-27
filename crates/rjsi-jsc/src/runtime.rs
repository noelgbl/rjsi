use javascriptcore_sys as jsc;
use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result, Runtime, Store};

pub struct JscPreparedKeyData {
    ctx: jsc::JSContextRef,
    val: jsc::JSValueRef,
}

impl JscPreparedKeyData {
    pub(crate) fn val(&self) -> jsc::JSValueRef {
        self.val
    }
}

impl Drop for JscPreparedKeyData {
    fn drop(&mut self) {
        unsafe {
            jsc::JSValueUnprotect(self.ctx, self.val);
        }
    }
}

pub(crate) struct JscGlobalContext {
    raw: jsc::JSGlobalContextRef,
}

impl JscGlobalContext {
    fn new() -> Self {
        let raw = unsafe { jsc::JSGlobalContextCreate(std::ptr::null_mut()) };
        Self { raw }
    }

    pub(crate) fn get_ref(&self) -> jsc::JSContextRef {
        self.raw as jsc::JSContextRef
    }
}

impl Drop for JscGlobalContext {
    fn drop(&mut self) {
        unsafe { jsc::JSGlobalContextRelease(self.raw) };
    }
}

pub struct JscRuntime {
    pub(crate) store: Store<crate::engine::JscEngine>,
    pub(crate) context: JscGlobalContext,
}

impl JscRuntime {
    pub fn new() -> Self {
        Self {
            store: Store::new(),
            context: JscGlobalContext::new(),
        }
    }

    pub fn prepare_key(
        &mut self,
        name: impl Into<String>,
    ) -> anyhow::Result<PreparedKey<crate::engine::JscEngine>> {
        let key = PreparedKey::new(name);
        self.ensure_prepared_key(key.id(), key.as_str());
        Ok(key)
    }

    fn ensure_prepared_key(&mut self, id: u64, name: &str) {
        if self.store.contains_prepared_key(id) {
            return;
        }

        let ctx = self.context.get_ref();
        let js_str = crate::engine::ManagedJSString::new(name);
        let val = unsafe { jsc::JSValueMakeString(ctx, js_str.0) };
        unsafe {
            jsc::JSValueProtect(ctx, val);
        }
        self.store
            .insert_prepared_key(id, JscPreparedKeyData { ctx, val });
    }
}

impl Default for JscRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime<crate::engine::JscEngine> for JscRuntime {
    fn with_scope<R>(
        &mut self,
        f: impl for<'js> FnOnce(&mut Context<'js, crate::engine::JscEngine>) -> R,
    ) -> R {
        let runtime_ptr = self as *mut _;
        let cx_raw = crate::engine::JscContext {
            ctx: self.context.get_ref(),
            runtime: runtime_ptr,
            pending_exception: None,
            _phantom: std::marker::PhantomData,
        };
        let mut cx = Context::new(cx_raw);
        f(&mut cx)
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.store.microtask_policy()
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.store.set_microtask_policy(policy);
    }
}

pub(crate) fn prepared_key<'js>(
    cx: &mut crate::engine::JscContext<'js>,
    key: &PreparedKey<crate::engine::JscEngine>,
) -> Result<crate::engine::JscKey<'js>> {
    if cx.runtime.is_null() {
        let js_str = crate::engine::ManagedJSString::new(key.as_str());
        let val = unsafe { jsc::JSValueMakeString(cx.ctx, js_str.0) };
        return Ok(crate::engine::JscKey::new(cx.ctx, val));
    }

    let runtime = unsafe { &mut *cx.runtime };
    runtime.ensure_prepared_key(key.id(), key.as_str());
    let prepared = runtime.store.get_prepared_key(key.id()).unwrap();
    Ok(crate::engine::JscKey::new(cx.ctx, prepared.val()))
}

pub(crate) struct JscClassHandle {
    raw: jsc::JSClassRef,
}

impl JscClassHandle {
    pub(crate) fn new(raw: jsc::JSClassRef) -> Self {
        Self { raw }
    }

    pub(crate) fn raw(&self) -> jsc::JSClassRef {
        self.raw
    }
}

impl Drop for JscClassHandle {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { jsc::JSClassRelease(self.raw) };
        }
    }
}
