//! Coercion helpers used by generated class bridges.

use crate::{error::E_TYPE, HostError, Runtime, ValueLike};

/// Accepted by bridge functions whose Rust signature takes `&[u8]`.
/// The macro generates extraction from JS values via [`BufferSource::from_js`].
///
/// Not a user-facing type — used only in generated code.
pub struct BufferSource(Vec<u8>);

impl BufferSource {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn from_js<'s, R: Runtime>(
        scope: &mut R::Scope<'s, '_>,
        val: R::Value<'s>,
    ) -> Result<Self, R::Error> {
        if let Some(bytes) = val.with_bytes(scope, |b| b.to_vec()) {
            return Ok(Self(bytes));
        }
        if let Some(s) = val.to_string_lossy(scope) {
            return Ok(Self(s.into_bytes()));
        }
        Err(HostError::type_error(E_TYPE, "expected BufferSource (ArrayBuffer, TypedArray, or string)")
            .into())
    }
}

/// Placeholder for passing another native instance as an argument (`NativeArg<Other>`).
pub struct NativeArg<T: crate::class::NativeClass>(pub T);
