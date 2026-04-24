//! Generational arena for storing JS thrown/rejected values across async boundaries.

use crate::JsValueImpl;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrownValueHandle {
    pub(crate) id: u32,
    pub(crate) generation: u32,
}

#[derive(Debug)]
pub struct ThrownValueStore<V: JsValueImpl> {
    slots: Vec<ThrownSlot<V>>,
    free: Vec<usize>,
}

#[derive(Debug)]
struct ThrownSlot<V: JsValueImpl> {
    generation: u32,
    value: Option<V>,
}

impl<V: JsValueImpl> ThrownValueStore<V> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: V) -> ThrownValueHandle {
        if let Some(id) = self.free.pop() {
            let slot = &mut self.slots[id];
            slot.generation = slot.generation.wrapping_add(1).max(1);
            slot.value = Some(value);
            return ThrownValueHandle {
                id: id as u32,
                generation: slot.generation,
            };
        }

        let id = self.slots.len();
        self.slots.push(ThrownSlot {
            generation: 1,
            value: Some(value),
        });
        ThrownValueHandle {
            id: id as u32,
            generation: 1,
        }
    }

    pub fn get(&self, handle: ThrownValueHandle) -> Option<V> {
        let id = handle.id as usize;
        let slot = self.slots.get(id)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.clone()
    }

    pub fn take(&mut self, handle: ThrownValueHandle) -> Option<V> {
        let id = handle.id as usize;
        let slot = self.slots.get_mut(id)?;
        if slot.generation != handle.generation {
            return None;
        }

        let value = slot.value.take();
        if value.is_some() {
            self.free.push(id);
        }
        value
    }
}
