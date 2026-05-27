// Pattern adapted from rlua / rquickjs.
// https://github.com/DelSkayn/rquickjs/blob/master/core/src/markers.rs

use std::marker::PhantomData;

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Invariant<'js>(PhantomData<&'js mut &'js fn(&'js ()) -> &'js ()>);

impl<'js> Invariant<'js> {
    pub const fn new() -> Self {
        Invariant(PhantomData)
    }
}
