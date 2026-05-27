use core::iter::FusedIterator;
use std::marker::PhantomData;

use crate::markers::Invariant;
use crate::{Context, Engine, Result, Value};

#[repr(transparent)]
pub struct Args<'js, E: Engine> {
    raw: E::RawArgs<'js>,
    _inv: PhantomData<Invariant<'js>>,
}

impl<'js, E: Engine> Args<'js, E> {
    pub fn new(raw: E::RawArgs<'js>) -> Self {
        Self {
            raw,
            _inv: PhantomData,
        }
    }

    pub fn as_raw(&self) -> &E::RawArgs<'js> {
        &self.raw
    }

    pub fn len(&self) -> usize {
        E::raw_args_len(&self.raw)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Value<'js, E>> {
        E::raw_args_get(&self.raw, index).map(Value::new)
    }

    pub fn iter(&self) -> ArgsIter<'_, 'js, E> {
        ArgsIter {
            raw: &self.raw,
            start: 0,
            end: E::raw_args_len(&self.raw),
        }
    }

    pub fn rest_from(&self, start: usize) -> ArgSlice<'_, 'js, E> {
        let end = E::raw_args_len(&self.raw);
        ArgSlice {
            raw: &self.raw,
            start: start.min(end),
            end,
        }
    }
}

pub struct ArgsIter<'a, 'js, E: Engine> {
    raw: &'a E::RawArgs<'js>,
    start: usize,
    end: usize,
}

impl<'a, 'js, E: Engine> Iterator for ArgsIter<'a, 'js, E> {
    type Item = E::Value<'js>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let idx = self.start;
        self.start += 1;
        E::raw_args_get(self.raw, idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end.saturating_sub(self.start);
        (n, Some(n))
    }
}

impl<'a, 'js, E: Engine> ExactSizeIterator for ArgsIter<'a, 'js, E> {
    fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl<'a, 'js, E: Engine> DoubleEndedIterator for ArgsIter<'a, 'js, E> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        E::raw_args_get(self.raw, self.end)
    }
}

impl<'a, 'js, E: Engine> FusedIterator for ArgsIter<'a, 'js, E> {}

impl<'a, 'js, E: Engine> IntoIterator for &'a Args<'js, E> {
    type Item = E::Value<'js>;
    type IntoIter = ArgsIter<'a, 'js, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ArgSlice<'a, 'js, E: Engine> {
    raw: &'a E::RawArgs<'js>,
    start: usize,
    end: usize,
}

impl<'a, 'js, E: Engine> ArgSlice<'a, 'js, E> {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Value<'js, E>> {
        let abs = self.start.checked_add(index).filter(|&i| i < self.end)?;
        E::raw_args_get(self.raw, abs).map(Value::new)
    }

    pub fn iter(&self) -> ArgsIter<'_, 'js, E> {
        ArgsIter {
            raw: self.raw,
            start: self.start,
            end: self.end,
        }
    }
}

impl<'a, 'js, E: Engine> IntoIterator for ArgSlice<'a, 'js, E> {
    type Item = E::Value<'js>;
    type IntoIter = ArgsIter<'a, 'js, E>;

    fn into_iter(self) -> Self::IntoIter {
        ArgsIter {
            raw: self.raw,
            start: self.start,
            end: self.end,
        }
    }
}

impl<'b, 'a, 'js, E: Engine> IntoIterator for &'b ArgSlice<'a, 'js, E> {
    type Item = E::Value<'js>;
    type IntoIter = ArgsIter<'b, 'js, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub trait RawHostFn<E: Engine> {
    fn call<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
        this: Value<'js, E>,
        args: Args<'js, E>,
    ) -> Result<Value<'js, E>>;
}

impl<E: Engine, F> RawHostFn<E> for F
where
    F: for<'js> FnMut(&mut Context<'js, E>, Value<'js, E>, Args<'js, E>) -> Result<Value<'js, E>>,
{
    fn call<'js>(
        &mut self,
        cx: &mut Context<'js, E>,
        this: Value<'js, E>,
        args: Args<'js, E>,
    ) -> Result<Value<'js, E>> {
        self(cx, this, args)
    }
}
