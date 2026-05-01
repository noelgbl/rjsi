use rjsi_core::{
    Context, InternKey, JsResult, Key, KeyCache, MicrotaskDrainPolicy, Runtime, StaticKeySlot,
};

use crate::engine::{runtime_ffi_ptr, HermesEngine};
use rusty_hermes::{PropNameId, Runtime as HermesRtInner};

pub struct HermesRuntime {
    pub inner: HermesRtInner,
    microtask_policy: MicrotaskDrainPolicy,
    static_slots: Vec<Option<String>>,
}

impl HermesRuntime {
    pub fn new() -> Result<Self, rusty_hermes::Error> {
        Ok(Self {
            inner: HermesRtInner::new()?,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            static_slots: Vec::new(),
        })
    }

    pub fn with_config(config: rusty_hermes::RuntimeConfig) -> Result<Self, rusty_hermes::Error> {
        Ok(Self {
            inner: HermesRtInner::with_config(config)?,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            static_slots: Vec::new(),
        })
    }
}

impl Default for HermesRuntime {
    fn default() -> Self {
        Self::new().expect("Hermes runtime creation failed")
    }
}

impl Runtime<HermesEngine> for HermesRuntime {
    fn with_scope<R>(&mut self, f: impl for<'rt> FnOnce(&mut Context<'rt, HermesEngine>) -> R) -> R {
        let ctx = crate::engine::HermesContext {
            inner: &mut self.inner,
        };
        let mut cx = Context::new(ctx);
        f(&mut cx)
    }

    fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.microtask_policy
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.microtask_policy = policy;
    }
}

impl InternKey<HermesEngine> for HermesRuntime {
    fn intern_str<'cx>(
        &mut self,
        cx: &mut Context<'cx, HermesEngine>,
        s: &str,
    ) -> JsResult<'cx, HermesEngine, Key<'cx, HermesEngine>> {
        let _ = self;
        let rt: &HermesRtInner = &*rjsi_core::__cx::context_mut(cx).inner;
        let p = PropNameId::from_utf8(rt, s);
        Ok(Key::new(unsafe { std::mem::transmute(p) }))
    }
}

impl KeyCache<HermesEngine> for HermesRuntime {
    fn get_or_intern<'cx>(
        &mut self,
        cx: &mut Context<'cx, HermesEngine>,
        slot: StaticKeySlot,
    ) -> JsResult<'cx, HermesEngine, Key<'cx, HermesEngine>> {
        let idx = slot.0 as usize;
        if idx >= self.static_slots.len() {
            self.static_slots.resize(idx + 1, None);
        }

        let s = if let Some(stored) = &self.static_slots[idx] {
            stored.clone()
        } else {
            let new_s = format!("__static_slot_{}", idx);
            self.static_slots[idx] = Some(new_s.clone());
            new_s
        };

        let rt: &HermesRtInner = &*rjsi_core::__cx::context_mut(cx).inner;
        let p = PropNameId::from_utf8(rt, s.as_str());
        Ok(Key::new(unsafe { std::mem::transmute(p) }))
    }
}

impl HermesRuntime {
    pub fn raw_hermes_rt(&self) -> *mut libhermes_sys::HermesRt {
        runtime_ffi_ptr(&self.inner)
    }
}
