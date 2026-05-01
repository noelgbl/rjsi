use rjsi_core::{
    Context, InternKey, JsResult, Key, KeyCache, MicrotaskDrainPolicy, Runtime, StaticKeySlot
};

pub struct JscRuntime {
    pub(crate) context: rusty_jsc::JSContext,
    microtask_policy: MicrotaskDrainPolicy,
    static_slots: Vec<Option<String>>,
}

impl JscRuntime {
    pub fn new() -> Self {
        Self {
            context: rusty_jsc::JSContext::new(),
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            static_slots: Vec::new(),
        }
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
        let cx_raw = crate::engine::JscContext {
            ctx: self.context.get_ref(),
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

impl InternKey<crate::engine::JscEngine> for JscRuntime {
    fn intern_str<'cx>(
        &mut self,
        cx: &mut Context<'cx, crate::engine::JscEngine>,
        s: &str,
    ) -> JsResult<'cx, crate::engine::JscEngine, Key<'cx, crate::engine::JscEngine>> {
        let js_str = crate::engine::ManagedJSString::new(s);
        let cx_raw = rjsi_core::__cx::context_mut(cx);
        let val = unsafe { rusty_jsc_sys::JSValueMakeString(cx_raw.ctx, js_str.0) };
        Ok(Key::new(crate::engine::JscKey::new(cx_raw.ctx, val)))
    }
}

impl KeyCache<crate::engine::JscEngine> for JscRuntime {
    fn get_or_intern<'cx>(
        &mut self,
        cx: &mut Context<'cx, crate::engine::JscEngine>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, crate::engine::JscEngine, Key<'cx, crate::engine::JscEngine>> {
        let idx = slot.0 as usize;
        if idx >= self.static_slots.len() {
            self.static_slots.resize(idx + 1, None);
        }

        let s = if let Some(s) = &self.static_slots[idx] {
            s.clone()
        } else {
            let new_s = format!("__static_slot_{}", idx);
            self.static_slots[idx] = Some(new_s.clone());
            new_s
        };

        self.intern_str(cx, &s)
    }
}
