use std::collections::HashMap;
use std::ops::DerefMut;

use boa_engine::{Context as BoaCx, JsString};
use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result, Runtime};

use crate::engine::BoaEngine;

pub struct BoaRuntime {
    prepared_keys: HashMap<u64, JsString>,
    pub(crate) context: BoaCx,
    microtask_policy: MicrotaskDrainPolicy,
}

impl BoaRuntime {
    pub fn new() -> Self {
        Self {
            prepared_keys: HashMap::new(),
            context: BoaCx::default(),
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        }
    }

    pub fn prepare_key(
        &mut self,
        name: impl Into<String>,
    ) -> anyhow::Result<PreparedKey<BoaEngine>> {
        let key = PreparedKey::new(name);
        self.ensure_prepared_key(key.id(), key.as_str());
        Ok(key)
    }

    fn ensure_prepared_key(&mut self, id: u64, name: &str) {
        if self.prepared_keys.contains_key(&id) {
            return;
        }

        let _ = self.context.interner_mut().get_or_intern(name);
        self.prepared_keys.insert(id, JsString::from(name));
    }
}

impl Default for BoaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime<BoaEngine> for BoaRuntime {
    fn with_scope<R>(&mut self, f: impl for<'rt> FnOnce(&mut Context<'rt, BoaEngine>) -> R) -> R {
        let runtime_ptr = self as *mut _;
        let wrapper = crate::engine::BoaContext {
            inner: &mut self.context,
            runtime: runtime_ptr,
        };
        let mut cx = Context::new(wrapper);
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
    cx: &mut crate::engine::BoaContext<'cx>,
    key: &PreparedKey<BoaEngine>,
) -> Result<JsString> {
    if cx.runtime.is_null() {
        return Ok(JsString::from(key.as_str()));
    }

    let runtime = unsafe { &mut *cx.runtime };
    runtime.ensure_prepared_key(key.id(), key.as_str());
    let boa_cx = cx.deref_mut();
    let _ = boa_cx.interner_mut().get_or_intern(key.as_str());
    Ok(runtime.prepared_keys.get(&key.id()).unwrap().clone())
}
