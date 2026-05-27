use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::marker::PhantomData;

use crate::{Engine, MicrotaskDrainPolicy};

pub struct Store<E: Engine> {
    prepared_keys: HashMap<u64, E::PreparedKeyData>,
    class_handles: HashMap<TypeId, Box<dyn Any>>,
    microtask_policy: MicrotaskDrainPolicy,
    _engine: PhantomData<fn() -> E>,
}

impl<E: Engine> Store<E> {
    pub fn new() -> Self {
        Self {
            prepared_keys: HashMap::new(),
            class_handles: HashMap::new(),
            microtask_policy: MicrotaskDrainPolicy::Explicit,
            _engine: PhantomData,
        }
    }

    pub fn microtask_policy(&self) -> MicrotaskDrainPolicy {
        self.microtask_policy
    }

    pub fn set_microtask_policy(&mut self, policy: MicrotaskDrainPolicy) {
        self.microtask_policy = policy;
    }

    pub fn get_prepared_key(&self, id: u64) -> Option<&E::PreparedKeyData> {
        self.prepared_keys.get(&id)
    }

    pub fn contains_prepared_key(&self, id: u64) -> bool {
        self.prepared_keys.contains_key(&id)
    }

    pub fn insert_prepared_key(&mut self, id: u64, data: E::PreparedKeyData) {
        self.prepared_keys.insert(id, data);
    }

    pub fn get_or_insert_prepared_key_with<F, Err>(
        &mut self,
        id: u64,
        build: F,
    ) -> Result<&E::PreparedKeyData, Err>
    where
        F: FnOnce() -> Result<E::PreparedKeyData, Err>,
    {
        match self.prepared_keys.entry(id) {
            Entry::Occupied(o) => Ok(o.into_mut()),
            Entry::Vacant(v) => {
                let data = build()?;
                Ok(v.insert(data))
            }
        }
    }

    pub fn get_class_handle<H: 'static>(&self, type_id: TypeId) -> Option<&H> {
        self.class_handles
            .get(&type_id)
            .and_then(|boxed| boxed.downcast_ref::<H>())
    }

    pub fn get_or_register_class_handle<H, F>(&mut self, type_id: TypeId, build: F) -> &H
    where
        H: 'static,
        F: FnOnce() -> H,
    {
        let boxed = self
            .class_handles
            .entry(type_id)
            .or_insert_with(|| Box::new(build()));
        boxed
            .downcast_ref::<H>()
            .expect("class handle type mismatch for TypeId")
    }

    pub fn get_or_try_register_class_handle<H, F, Err>(
        &mut self,
        type_id: TypeId,
        build: F,
    ) -> Result<&H, Err>
    where
        H: 'static,
        F: FnOnce() -> Result<H, Err>,
    {
        let boxed = match self.class_handles.entry(type_id) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let handle = build()?;
                v.insert(Box::new(handle))
            }
        };
        Ok(boxed
            .downcast_ref::<H>()
            .expect("class handle type mismatch for TypeId"))
    }
}

impl<E: Engine> Default for Store<E> {
    fn default() -> Self {
        Self::new()
    }
}
