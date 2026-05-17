use core::iter::FusedIterator;

use crate::{Context, Engine, Result, Value};

#[repr(transparent)]
pub struct Args<'cx, E: Engine> {
    raw: E::RawArgs<'cx>,
}

impl<'cx, E: Engine> Args<'cx, E> {
    pub fn new(raw: E::RawArgs<'cx>) -> Self {
        Self { raw }
    }

    pub fn as_raw(&self) -> &E::RawArgs<'cx> {
        &self.raw
    }

    pub fn len(&self) -> usize {
        E::raw_args_len(&self.raw)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Value<'cx, E>> {
        E::raw_args_get(&self.raw, index).map(Value::new)
    }

    pub fn iter(&self) -> ArgsIter<'_, 'cx, E> {
        ArgsIter {
            raw: &self.raw,
            start: 0,
            end: E::raw_args_len(&self.raw),
        }
    }

    pub fn rest_from(&self, start: usize) -> ArgSlice<'_, 'cx, E> {
        let end = E::raw_args_len(&self.raw);
        ArgSlice {
            raw: &self.raw,
            start: start.min(end),
            end,
        }
    }
}

pub struct ArgsIter<'a, 'cx, E: Engine> {
    raw: &'a E::RawArgs<'cx>,
    start: usize,
    end: usize,
}

impl<'a, 'cx, E: Engine> Iterator for ArgsIter<'a, 'cx, E> {
    type Item = E::Value<'cx>;

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

impl<'a, 'cx, E: Engine> ExactSizeIterator for ArgsIter<'a, 'cx, E> {
    fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl<'a, 'cx, E: Engine> DoubleEndedIterator for ArgsIter<'a, 'cx, E> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        self.end -= 1;
        E::raw_args_get(self.raw, self.end)
    }
}

impl<'a, 'cx, E: Engine> FusedIterator for ArgsIter<'a, 'cx, E> {}

impl<'a, 'cx, E: Engine> IntoIterator for &'a Args<'cx, E> {
    type Item = E::Value<'cx>;
    type IntoIter = ArgsIter<'a, 'cx, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ArgSlice<'a, 'cx, E: Engine> {
    raw: &'a E::RawArgs<'cx>,
    start: usize,
    end: usize,
}

impl<'a, 'cx, E: Engine> ArgSlice<'a, 'cx, E> {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Value<'cx, E>> {
        let abs = self.start.checked_add(index).filter(|&i| i < self.end)?;
        E::raw_args_get(self.raw, abs).map(Value::new)
    }

    pub fn iter(&self) -> ArgsIter<'_, 'cx, E> {
        ArgsIter {
            raw: self.raw,
            start: self.start,
            end: self.end,
        }
    }
}

impl<'a, 'cx, E: Engine> IntoIterator for ArgSlice<'a, 'cx, E> {
    type Item = E::Value<'cx>;
    type IntoIter = ArgsIter<'a, 'cx, E>;

    fn into_iter(self) -> Self::IntoIter {
        ArgsIter {
            raw: self.raw,
            start: self.start,
            end: self.end,
        }
    }
}

impl<'b, 'a, 'cx, E: Engine> IntoIterator for &'b ArgSlice<'a, 'cx, E> {
    type Item = E::Value<'cx>;
    type IntoIter = ArgsIter<'b, 'cx, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub trait RawHostFn<E: Engine> {
    fn call<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: Args<'rt, E>,
    ) -> Result<Value<'rt, E>>;
}

impl<E: Engine, F> RawHostFn<E> for F
where
    F: for<'rt> FnMut(&mut Context<'rt, E>, Value<'rt, E>, Args<'rt, E>) -> Result<Value<'rt, E>>,
{
    fn call<'rt>(
        &mut self,
        cx: &mut Context<'rt, E>,
        this: Value<'rt, E>,
        args: Args<'rt, E>,
    ) -> Result<Value<'rt, E>> {
        self(cx, this, args)
    }
}
