use crate::{Context, Engine, Error, Object, Result, Value};

/// The execution state of a promise.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum PromiseState {
    /// The promise has not yet completed.
    Pending,
    /// The promise completed succefully.
    Resolved,
    /// The promise completed with an error.
    Rejected,
}

/// A JavaScript promise.
#[repr(transparent)]
pub struct Promise<'js, E: Engine>(pub(crate) Object<'js, E>);

impl<'js, E: Engine> Promise<'js, E> {
    pub fn new(obj: Object<'js, E>) -> Self {
        Self(obj)
    }

    pub fn into_object(self) -> Object<'js, E> {
        self.0
    }

    pub fn as_object(&self) -> &Object<'js, E> {
        &self.0
    }
}

pub trait Promises: Engine {
    fn promise_new<'rt>(
        cx: &mut Context<'rt, Self>,
    ) -> Result<(Self::Object<'rt>, Self::Object<'rt>)>;

    fn promise_resolve<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        value: Self::Value<'rt>,
    ) -> Result<()>;

    fn promise_reject<'rt>(
        cx: &mut Context<'rt, Self>,
        resolver: Self::Object<'rt>,
        reason: Self::Value<'rt>,
    ) -> Result<()>;

    fn promise_state<'rt>(
        cx: &mut Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<PromiseState>;

    fn promise_result<'rt>(
        cx: &mut Context<'rt, Self>,
        promise: &Self::Object<'rt>,
    ) -> Result<Option<std::result::Result<Self::Value<'rt>, Self::Value<'rt>>>>;
}

/// Engines that allow manual manipulation of the microtask queue.
pub trait Microtasks: Engine {
    /// Enqueues a function to be run as a microtask.
    fn queue_microtask<'rt>(cx: &mut Context<'rt, Self>, task: Self::Function<'rt>);

    /// Drains the microtask queue synchronously.
    fn drain_microtasks<'rt>(cx: &mut Context<'rt, Self>);
}

pub trait Modules: Engine + Promises {
    fn install_module_host(
        runtime: &mut Self::Runtime,
        host: crate::module::ModuleHost,
    ) -> Result<()>;

    fn set_import_meta_hook(
        runtime: &mut Self::Runtime,
        hook: crate::module::ImportMetaHook,
    ) -> Result<()>;

    fn module_evaluate<'rt>(
        cx: &mut Context<'rt, Self>,
        name: &str,
        src: &str,
    ) -> Result<Self::Object<'rt>>;

    fn module_import<'rt>(
        cx: &mut Context<'rt, Self>,
        specifier: &str,
    ) -> Result<Self::Object<'rt>>;
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum TypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

impl TypedArrayKind {
    pub const fn element_size(self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 | Self::Uint8Clamped => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Float64 | Self::BigInt64 | Self::BigUint64 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypedArrayInfo {
    pub kind: TypedArrayKind,
    pub byte_offset: usize,
    pub byte_length: usize,
    pub length: usize,
}

pub type BufferOwner = Box<dyn std::any::Any + Send + 'static>;

pub trait Buffers: Engine {
    fn value_is_array_buffer<'cx>(val: &Self::Value<'cx>) -> bool;

    fn value_typed_array_kind<'cx>(val: &Self::Value<'cx>) -> Option<TypedArrayKind>;

    unsafe fn array_buffer_adopt<'rt>(
        cx: &mut Context<'rt, Self>,
        ptr: *mut u8,
        len: usize,
        owner: BufferOwner,
    ) -> Result<Self::Object<'rt>>;

    fn array_buffer_alloc<'rt>(
        cx: &mut Context<'rt, Self>,
        len: usize,
    ) -> Result<Self::Object<'rt>>;

    fn typed_array_new<'rt>(
        cx: &mut Context<'rt, Self>,
        kind: TypedArrayKind,
        buffer: Self::Object<'rt>,
        byte_offset: usize,
        length: usize,
    ) -> Result<Self::Object<'rt>>;

    fn array_buffer_byte_length<'cx>(
        cx: &mut Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<usize>;

    fn typed_array_info<'cx>(
        cx: &mut Context<'cx, Self>,
        obj: &Self::Object<'cx>,
    ) -> Result<TypedArrayInfo>;

    fn typed_array_buffer<'rt>(
        cx: &mut Context<'rt, Self>,
        obj: &Self::Object<'rt>,
    ) -> Result<Self::Object<'rt>>;

    fn array_buffer_copy_to<'cx>(
        cx: &mut Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()>;

    fn typed_array_copy_to<'cx>(
        cx: &mut Context<'cx, Self>,
        obj: &Self::Object<'cx>,
        dst: &mut [u8],
    ) -> Result<()>;
}

#[repr(transparent)]
pub struct ArrayBuffer<'cx, E: Engine>(pub(crate) Object<'cx, E>);

impl<'cx, E: Engine> ArrayBuffer<'cx, E> {
    pub fn new(obj: Object<'cx, E>) -> Self {
        Self(obj)
    }

    pub fn as_object(&self) -> &Object<'cx, E> {
        &self.0
    }

    pub fn into_object(self) -> Object<'cx, E> {
        self.0
    }

    pub fn into_value(self) -> Value<'cx, E> {
        self.0.into_value()
    }
}

impl<'cx, E: Buffers> ArrayBuffer<'cx, E> {
    pub fn byte_length(&self, cx: &mut Context<'cx, E>) -> Result<usize> {
        E::array_buffer_byte_length(cx, &self.0.raw)
    }

    pub fn append_to(&self, cx: &mut Context<'cx, E>, out: &mut Vec<u8>) -> Result<()> {
        let len = self.byte_length(cx)?;
        let start = out.len();
        out.resize(start + len, 0);
        E::array_buffer_copy_to(cx, &self.0.raw, &mut out[start..])
    }

    pub fn to_vec(&self, cx: &mut Context<'cx, E>) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        self.append_to(cx, &mut v)?;
        Ok(v)
    }
}

macro_rules! typed_array_wrapper {
    ($name:ident, $elt:ty, $kind:ident) => {
        #[doc = concat!("A JavaScript `", stringify!($name), "` view.")]
        #[repr(transparent)]
        pub struct $name<'cx, E: Engine>(pub(crate) Object<'cx, E>);

        impl<'cx, E: Engine> $name<'cx, E> {
            pub fn new(obj: Object<'cx, E>) -> Self {
                Self(obj)
            }

            pub fn as_object(&self) -> &Object<'cx, E> {
                &self.0
            }

            pub fn into_object(self) -> Object<'cx, E> {
                self.0
            }

            pub const fn kind() -> TypedArrayKind {
                TypedArrayKind::$kind
            }

            pub fn into_value(self) -> Value<'cx, E> {
                self.0.into_value()
            }
        }

        impl<'cx, E: Buffers> $name<'cx, E> {
            pub fn info(&self, cx: &mut Context<'cx, E>) -> Result<TypedArrayInfo> {
                E::typed_array_info(cx, &self.0.raw)
            }

            pub fn length(&self, cx: &mut Context<'cx, E>) -> Result<usize> {
                Ok(self.info(cx)?.length)
            }

            pub fn byte_length(&self, cx: &mut Context<'cx, E>) -> Result<usize> {
                Ok(self.info(cx)?.byte_length)
            }

            pub fn buffer(&self, cx: &mut Context<'cx, E>) -> Result<ArrayBuffer<'cx, E>> {
                let raw = E::typed_array_buffer(cx, &self.0.raw)?;
                Ok(ArrayBuffer(Object::new(raw)))
            }

            pub fn append_to(&self, cx: &mut Context<'cx, E>, out: &mut Vec<$elt>) -> Result<()> {
                let info = self.info(cx)?;
                if info.kind != TypedArrayKind::$kind {
                    return Err(Error::type_err(concat!(
                        stringify!($name),
                        ": unexpected typed array kind"
                    )));
                }
                let elt_size = std::mem::size_of::<$elt>();
                if info.byte_length % elt_size != 0 {
                    return Err(Error::type_err(concat!(
                        stringify!($name),
                        ": byte_length not a multiple of element_size"
                    )));
                }
                let elt_count = info.byte_length / elt_size;
                let start = out.len();
                out.reserve(elt_count);
                let dst_bytes = unsafe {
                    std::slice::from_raw_parts_mut(
                        out.as_mut_ptr().add(start) as *mut u8,
                        elt_count * elt_size,
                    )
                };
                E::typed_array_copy_to(cx, &self.0.raw, dst_bytes)?;
                unsafe { out.set_len(start + elt_count) };
                Ok(())
            }

            pub fn to_vec(&self, cx: &mut Context<'cx, E>) -> Result<Vec<$elt>> {
                let mut v = Vec::new();
                self.append_to(cx, &mut v)?;
                Ok(v)
            }
        }
    };
}

typed_array_wrapper!(Int8Array, i8, Int8);
typed_array_wrapper!(Uint8Array, u8, Uint8);
typed_array_wrapper!(Uint8ClampedArray, u8, Uint8Clamped);
typed_array_wrapper!(Int16Array, i16, Int16);
typed_array_wrapper!(Uint16Array, u16, Uint16);
typed_array_wrapper!(Int32Array, i32, Int32);
typed_array_wrapper!(Uint32Array, u32, Uint32);
typed_array_wrapper!(Float32Array, f32, Float32);
typed_array_wrapper!(Float64Array, f64, Float64);
typed_array_wrapper!(BigInt64Array, i64, BigInt64);
typed_array_wrapper!(BigUint64Array, u64, BigUint64);
