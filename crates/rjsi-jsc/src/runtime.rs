use std::collections::HashMap;

use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result, Runtime};

pub struct JscPreparedKeyData {
    ctx: rusty_jsc_sys::JSContextRef,
    val: rusty_jsc_sys::JSValueRef,
}

impl Drop for JscPreparedKeyData {
    fn drop(&mut self) {
        unsafe {
            rusty_jsc_sys::JSValueUnprotect(self.ctx, self.val);
        }
    }
}

pub struct JscRuntime {
    prepared_keys: HashMap<u64, JscPreparedKeyData>,
    pub(crate) context: rusty_jsc::JSContext,
    microtask_policy: MicrotaskDrainPolicy,
}

impl JscRuntime {
    pub fn new() -> Self {
        Self {
            prepared_keys: HashMap::new(),
            context: rusty_jsc::JSContext::new(),
            microtask_policy: MicrotaskDrainPolicy::Explicit,
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
        if self.prepared_keys.contains_key(&id) {
            return;
        }

        let ctx = self.context.get_ref();
        let js_str = crate::engine::ManagedJSString::new(name);
        let val = unsafe { rusty_jsc_sys::JSValueMakeString(ctx, js_str.0) };
        unsafe {
            rusty_jsc_sys::JSValueProtect(ctx, val);
        }
        self.prepared_keys
            .insert(id, JscPreparedKeyData { ctx, val });
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
        f: impl for<'rt> FnOnce(&mut Context<'rt, crate::engine::JscEngine>) -> R,
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
        self.microtask_policy
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.microtask_policy = policy;
    }
}

pub(crate) fn prepared_key<'cx>(
    cx: &mut crate::engine::JscContext<'cx>,
    key: &PreparedKey<crate::engine::JscEngine>,
) -> Result<crate::engine::JscKey<'cx>> {
    if cx.runtime.is_null() {
        let js_str = crate::engine::ManagedJSString::new(key.as_str());
        let val = unsafe { rusty_jsc_sys::JSValueMakeString(cx.ctx, js_str.0) };
        return Ok(crate::engine::JscKey::new(cx.ctx, val));
    }

    let runtime = unsafe { &mut *cx.runtime };
    runtime.ensure_prepared_key(key.id(), key.as_str());
    let prepared = runtime.prepared_keys.get(&key.id()).unwrap();
    Ok(crate::engine::JscKey::new(cx.ctx, prepared.val))
}
