use std::collections::HashMap;

use rjsi_core::{Context, JsResult, MicrotaskDrainPolicy, PreparedKey, Runtime};
use rusty_hermes::{PropNameId, Runtime as HermesRtInner};

use crate::engine::{HermesEngine, runtime_ffi_ptr};

pub struct HermesPreparedKeyData {
    key: PropNameId<'static>,
}

pub struct HermesRuntime {
    prepared_keys: HashMap<u64, HermesPreparedKeyData>,
    pub inner: HermesRtInner,
    microtask_policy: MicrotaskDrainPolicy,
}

impl HermesRuntime {
    pub fn new() -> Result<Self, rusty_hermes::Error> {
        Ok(Self {
            prepared_keys: HashMap::new(),
            inner: HermesRtInner::new()?,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        })
    }

    pub fn with_config(config: rusty_hermes::RuntimeConfig) -> Result<Self, rusty_hermes::Error> {
        Ok(Self {
            prepared_keys: HashMap::new(),
            inner: HermesRtInner::with_config(config)?,
            microtask_policy: MicrotaskDrainPolicy::Explicit,
        })
    }

    pub fn prepare_key(
        &mut self,
        name: impl Into<String>,
    ) -> anyhow::Result<PreparedKey<HermesEngine>> {
        let key = PreparedKey::new(name);
        self.ensure_prepared_key(key.id(), key.as_str());
        Ok(key)
    }

    fn ensure_prepared_key(&mut self, id: u64, name: &str) {
        if self.prepared_keys.contains_key(&id) {
            return;
        }

        let key = PropNameId::from_utf8(&self.inner, name);
        self.prepared_keys.insert(
            id,
            HermesPreparedKeyData {
                key: unsafe { std::mem::transmute(key) },
            },
        );
    }
}

impl Default for HermesRuntime {
    fn default() -> Self {
        Self::new().expect("Hermes runtime creation failed")
    }
}

impl Runtime<HermesEngine> for HermesRuntime {
    fn with_scope<R>(
        &mut self,
        f: impl for<'rt> FnOnce(&mut Context<'rt, HermesEngine>) -> R,
    ) -> R {
        let runtime_ptr = self as *mut _;
        let ctx = crate::engine::HermesContext {
            inner: &mut self.inner,
            runtime: runtime_ptr,
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

pub(crate) fn prepared_key<'cx>(
    cx: &mut crate::engine::HermesContext<'cx>,
    key: &PreparedKey<HermesEngine>,
) -> JsResult<'cx, HermesEngine, PropNameId<'cx>> {
    if cx.runtime.is_null() {
        let prepared = PropNameId::from_utf8(&*cx.inner, key.as_str());
        return Ok(unsafe { std::mem::transmute(prepared) });
    }

    let runtime = unsafe { &mut *cx.runtime };
    runtime.ensure_prepared_key(key.id(), key.as_str());
    let prepared = &runtime.prepared_keys.get(&key.id()).unwrap().key;
    Ok(unsafe { std::mem::transmute_copy(prepared) })
}

impl HermesRuntime {
    pub fn raw_hermes_rt(&self) -> *mut libhermes_sys::HermesRt {
        runtime_ffi_ptr(&self.inner)
    }
}
