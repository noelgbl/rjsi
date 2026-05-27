use std::mem::ManuallyDrop;
use std::ops::Deref;

use hermes::{PropNameId, Runtime as HermesRtInner};
use hermes_sys::{HermesRt, hermes__PropNameID__Release};
use rjsi_core::{Context, MicrotaskDrainPolicy, PreparedKey, Result as RjsiResult, Runtime, Store};

use crate::engine::{HermesEngine, runtime_ffi_ptr};

pub struct HermesPreparedKeyData {
    pv: *mut std::ffi::c_void,
    #[allow(dead_code)]
    rt: *mut HermesRt,
    _marker: std::marker::PhantomData<&'static ()>,
}

impl HermesPreparedKeyData {
    fn from_owned<'rt>(owned: PropNameId<'rt>) -> Self {
        let cached =
            unsafe { std::mem::transmute_copy::<PropNameId<'rt>, HermesPreparedKeyData>(&owned) };
        std::mem::forget(owned);
        cached
    }
}

impl Drop for HermesPreparedKeyData {
    fn drop(&mut self) {
        if !self.pv.is_null() {
            unsafe { hermes__PropNameID__Release(self.pv) };
        }
    }
}

pub enum HermesKey<'js> {
    Owned(PropNameId<'js>),
    Borrowed(ManuallyDrop<PropNameId<'js>>),
}

impl<'js> Deref for HermesKey<'js> {
    type Target = PropNameId<'js>;

    fn deref(&self) -> &Self::Target {
        match self {
            HermesKey::Owned(p) => p,
            HermesKey::Borrowed(p) => p,
        }
    }
}

unsafe fn borrow_propname_id<'js>(cached: &HermesPreparedKeyData) -> ManuallyDrop<PropNameId<'js>> {
    ManuallyDrop::new(unsafe {
        std::mem::transmute_copy::<HermesPreparedKeyData, PropNameId<'js>>(cached)
    })
}

pub struct HermesRuntime {
    pub(crate) store: Store<HermesEngine>,
    pub inner: HermesRtInner,
}

impl HermesRuntime {
    pub fn new() -> Result<Self, hermes::Error> {
        Ok(Self {
            store: Store::new(),
            inner: HermesRtInner::new()?,
        })
    }

    pub fn with_config(config: hermes::RuntimeConfig) -> Result<Self, hermes::Error> {
        Ok(Self {
            store: Store::new(),
            inner: HermesRtInner::with_config(config)?,
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
        if self.store.contains_prepared_key(id) {
            return;
        }
        let owned = PropNameId::from_utf8(&self.inner, name);
        self.store
            .insert_prepared_key(id, HermesPreparedKeyData::from_owned(owned));
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
        f: impl for<'js> FnOnce(&mut Context<'js, HermesEngine>) -> R,
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
        self.store.microtask_policy()
    }

    fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.store.set_microtask_policy(policy);
    }
}

pub(crate) fn prepared_key<'js>(
    cx: &mut crate::engine::HermesContext<'js>,
    key: &PreparedKey<HermesEngine>,
) -> RjsiResult<HermesKey<'js>> {
    if cx.runtime.is_null() {
        let p = PropNameId::from_utf8(&*cx.inner, key.as_str());
        return Ok(HermesKey::Owned(unsafe { std::mem::transmute(p) }));
    }

    let runtime = unsafe { &mut *cx.runtime };
    runtime.ensure_prepared_key(key.id(), key.as_str());
    let cached = runtime.store.get_prepared_key(key.id()).unwrap();
    Ok(HermesKey::Borrowed(unsafe { borrow_propname_id(cached) }))
}

impl HermesRuntime {
    pub fn raw_hermes_rt(&self) -> *mut hermes_sys::HermesRt {
        runtime_ffi_ptr(&self.inner)
    }
}
