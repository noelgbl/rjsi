#![allow(non_upper_case_globals)]

mod array;
mod array_buffer;
mod bigint;
mod convert;
mod error;
mod function;
mod object;
mod prepared_js;
mod propnameid;
mod scope;
mod string;
mod symbol;
mod value;
mod weak_object;

use std::marker::PhantomData;

pub use array::Array;
pub use array_buffer::ArrayBuffer;
pub use bigint::BigInt;
pub use convert::{FromJs, IntoJs};
pub use error::{Error, Result};
pub use function::Function;
use hermes_sys::*;
pub use hermes_sys::{
    HermesFatalHandler, HermesHostObjectFinalizer, HermesHostObjectGetCallback, HermesHostObjectGetPropertyNamesCallback, HermesHostObjectSetCallback, HermesNativeStateFinalizer, HermesRuntimeConfig
};
pub use object::Object;
pub use prepared_js::PreparedJavaScript;
pub use propnameid::PropNameId;
pub use scope::Scope;
pub use string::JsString;
pub use symbol::Symbol;
pub use value::{Value, ValueKind};
pub use weak_object::WeakObject;

#[doc(hidden)]
pub mod __private {
    pub use hermes_sys::{
        HermesHostFunctionCallback, HermesRt, HermesValue, HermesValueData, HermesValueKind_Undefined, hermes__Function__CreateFromHostFunction, hermes__Function__Release, hermes__Object__Release, hermes__Object__SetProperty__String, hermes__PropNameID__ForUtf8, hermes__PropNameID__Release, hermes__Runtime__Global, hermes__Runtime__HasPendingError, hermes__Runtime__SetPendingErrorMessage, hermes__String__CreateFromUtf8, hermes__String__Release
    };

    pub use crate::error::Error;
    pub use crate::function::{FromJsArg, IntoJsRet};

    pub fn undefined_value() -> HermesValue {
        HermesValue {
            kind: HermesValueKind_Undefined,
            data: HermesValueData { number: 0.0 },
        }
    }

    pub unsafe fn set_error_and_return_undefined(rt: *mut HermesRt, err: &Error) -> HermesValue {
        unsafe {
            let msg = err.to_string();
            hermes__Runtime__SetPendingErrorMessage(rt, msg.as_ptr(), msg.len());
            undefined_value()
        }
    }

    pub unsafe extern "C" fn noop_finalizer(_: *mut std::ffi::c_void) {}
}

pub struct RuntimeConfig {
    raw: HermesRuntimeConfig,
}

impl RuntimeConfig {
    pub fn builder() -> RuntimeConfigBuilder {
        RuntimeConfigBuilder {
            raw: HermesRuntimeConfig {
                enable_eval: true,
                es6_proxy: true,
                intl: true,
                microtask_queue: false,
                enable_generator: true,
                enable_block_scoping: false,
                enable_hermes_internal: true,
                enable_hermes_internal_test_methods: false,
                max_num_registers: 128 * 1024,
                enable_jit: false,
                force_jit: false,
                jit_threshold: 1 << 5,
                jit_memory_limit: 32 << 20,
                enable_async_generators: false,
                bytecode_warmup_percent: 0,
                randomize_memory_layout: false,
            },
        }
    }
}

pub struct RuntimeConfigBuilder {
    raw: HermesRuntimeConfig,
}

impl RuntimeConfigBuilder {
    pub fn enable_eval(mut self, v: bool) -> Self {
        self.raw.enable_eval = v;
        self
    }

    pub fn es6_proxy(mut self, v: bool) -> Self {
        self.raw.es6_proxy = v;
        self
    }

    pub fn intl(mut self, v: bool) -> Self {
        self.raw.intl = v;
        self
    }

    pub fn microtask_queue(mut self, v: bool) -> Self {
        self.raw.microtask_queue = v;
        self
    }

    pub fn enable_generator(mut self, v: bool) -> Self {
        self.raw.enable_generator = v;
        self
    }

    pub fn enable_block_scoping(mut self, v: bool) -> Self {
        self.raw.enable_block_scoping = v;
        self
    }

    pub fn enable_hermes_internal(mut self, v: bool) -> Self {
        self.raw.enable_hermes_internal = v;
        self
    }

    pub fn enable_hermes_internal_test_methods(mut self, v: bool) -> Self {
        self.raw.enable_hermes_internal_test_methods = v;
        self
    }

    pub fn max_num_registers(mut self, v: u32) -> Self {
        self.raw.max_num_registers = v;
        self
    }

    pub fn enable_jit(mut self, v: bool) -> Self {
        self.raw.enable_jit = v;
        self
    }

    pub fn force_jit(mut self, v: bool) -> Self {
        self.raw.force_jit = v;
        self
    }

    pub fn jit_threshold(mut self, v: u32) -> Self {
        self.raw.jit_threshold = v;
        self
    }

    pub fn jit_memory_limit(mut self, v: u32) -> Self {
        self.raw.jit_memory_limit = v;
        self
    }

    pub fn enable_async_generators(mut self, v: bool) -> Self {
        self.raw.enable_async_generators = v;
        self
    }

    pub fn bytecode_warmup_percent(mut self, v: u32) -> Self {
        self.raw.bytecode_warmup_percent = v;
        self
    }

    pub fn randomize_memory_layout(mut self, v: bool) -> Self {
        self.raw.randomize_memory_layout = v;
        self
    }

    pub fn build(self) -> RuntimeConfig {
        RuntimeConfig { raw: self.raw }
    }
}

pub struct Runtime {
    pub(crate) raw: *mut HermesRt,
    _not_send_sync: PhantomData<*mut ()>,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let raw = unsafe { hermes__Runtime__New() };
        if raw.is_null() {
            return Err(Error::RuntimeError(
                "failed to create Hermes runtime".into(),
            ));
        }
        Ok(Runtime {
            raw,
            _not_send_sync: PhantomData,
        })
    }

    pub fn with_config(config: RuntimeConfig) -> Result<Self> {
        let raw = unsafe { hermes__Runtime__NewWithConfig(&config.raw) };
        if raw.is_null() {
            return Err(Error::RuntimeError(
                "failed to create Hermes runtime".into(),
            ));
        }
        Ok(Runtime {
            raw,
            _not_send_sync: PhantomData,
        })
    }

    pub fn eval(&self, code: &str) -> Result<Value<'_>> {
        self.eval_with_url(code, "<eval>")
    }

    pub fn eval_with_url(&self, code: &str, url: &str) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__EvaluateJavaScript(
                self.raw,
                code.as_ptr(),
                code.len(),
                url.as_ptr() as *const i8,
                url.len(),
            )
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    pub fn global(&self) -> Object<'_> {
        let pv = unsafe { hermes__Runtime__Global(self.raw) };
        Object {
            pv,
            rt: self.raw,
            _marker: PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn __register_op(
        &self,
        name: &str,
        param_count: u32,
        callback: __private::HermesHostFunctionCallback,
    ) -> Result<()> {
        let name_pv = unsafe { hermes__PropNameID__ForUtf8(self.raw, name.as_ptr(), name.len()) };
        let func_pv = unsafe {
            hermes__Function__CreateFromHostFunction(
                self.raw,
                name_pv,
                param_count,
                callback,
                std::ptr::null_mut(),
                __private::noop_finalizer,
            )
        };
        unsafe { hermes__PropNameID__Release(name_pv) };
        error::check_error(self.raw)?;

        let global_pv = unsafe { hermes__Runtime__Global(self.raw) };
        let key_pv = unsafe { hermes__String__CreateFromUtf8(self.raw, name.as_ptr(), name.len()) };
        let val = HermesValue {
            kind: HermesValueKind_Object,
            data: HermesValueData { pointer: func_pv },
        };
        unsafe {
            hermes__Object__SetProperty__String(self.raw, global_pv, key_pv, &val);
            hermes__String__Release(key_pv);
            hermes__Object__Release(global_pv);
            hermes__Function__Release(func_pv);
        }
        Ok(())
    }

    pub fn drain_microtasks(&self) -> Result<bool> {
        let rc = unsafe { hermes__Runtime__DrainMicrotasks(self.raw, -1) };
        if rc < 0 {
            error::check_error(self.raw)?;
        }
        Ok(rc == 1)
    }

    pub fn create_value_from_json(&self, json: &str) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__CreateValueFromJsonUtf8(self.raw, json.as_ptr(), json.len())
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    pub fn eval_with_source_map(
        &self,
        code: &str,
        source_map: &[u8],
        url: &str,
    ) -> Result<Value<'_>> {
        let raw = unsafe {
            hermes__Runtime__EvaluateJavaScriptWithSourceMap(
                self.raw,
                code.as_ptr(),
                code.len(),
                source_map.as_ptr(),
                source_map.len(),
                url.as_ptr() as *const i8,
                url.len(),
            )
        };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    pub fn prepare_javascript(&self, code: &str, url: &str) -> Result<PreparedJavaScript> {
        let raw = unsafe {
            hermes__Runtime__PrepareJavaScript(
                self.raw,
                code.as_ptr(),
                code.len(),
                url.as_ptr() as *const i8,
                url.len(),
            )
        };
        error::check_error(self.raw)?;
        if raw.is_null() {
            return Err(Error::RuntimeError("failed to prepare JavaScript".into()));
        }
        Ok(PreparedJavaScript { raw })
    }

    pub fn evaluate_prepared_javascript(&self, prepared: &PreparedJavaScript) -> Result<Value<'_>> {
        let raw = unsafe { hermes__Runtime__EvaluatePreparedJavaScript(self.raw, prepared.raw) };
        error::check_error(self.raw)?;
        Ok(unsafe { Value::from_raw(self.raw, raw) })
    }

    pub fn description(&self) -> String {
        let mut buf = vec![0u8; 256];
        let len = unsafe {
            hermes__Runtime__Description(self.raw, buf.as_mut_ptr() as *mut i8, buf.len())
        };
        buf.truncate(len);
        String::from_utf8_lossy(&buf).into_owned()
    }

    pub fn is_inspectable(&self) -> bool {
        unsafe { hermes__Runtime__IsInspectable(self.raw) }
    }

    pub fn watch_time_limit(&self, timeout_ms: u32) {
        unsafe { hermes__Runtime__WatchTimeLimit(self.raw, timeout_ms) }
    }

    pub fn unwatch_time_limit(&self) {
        unsafe { hermes__Runtime__UnwatchTimeLimit(self.raw) }
    }

    pub fn async_trigger_timeout(&self) {
        unsafe { hermes__Runtime__AsyncTriggerTimeout(self.raw) }
    }

    pub fn queue_microtask(&self, func: &Function<'_>) -> Result<()> {
        let ok = unsafe { hermes__Runtime__QueueMicrotask(self.raw, func.pv) };
        if !ok {
            return error::check_error(self.raw).map(|_| ());
        }
        Ok(())
    }

    pub fn register_for_profiling(&self) {
        unsafe { hermes__Runtime__RegisterForProfiling(self.raw) }
    }

    pub fn unregister_for_profiling(&self) {
        unsafe { hermes__Runtime__UnregisterForProfiling(self.raw) }
    }

    pub fn load_segment(&self, data: &[u8], context: &Value<'_>) -> Result<()> {
        let ok = unsafe {
            hermes__Runtime__LoadSegment(self.raw, data.as_ptr(), data.len(), &context.raw)
        };
        if !ok {
            return error::check_error(self.raw).map(|_| ());
        }
        Ok(())
    }

    pub fn reset_timezone_cache(&self) {
        unsafe { hermes__Runtime__ResetTimezoneCache(self.raw) }
    }

    pub fn is_hermes_bytecode(data: &[u8]) -> bool {
        unsafe { hermes__IsHermesBytecode(data.as_ptr(), data.len()) }
    }

    pub fn bytecode_version() -> u32 {
        unsafe { hermes__GetBytecodeVersion() }
    }

    pub fn bytecode_sanity_check(data: &[u8]) -> bool {
        unsafe { hermes__HermesBytecodeSanityCheck(data.as_ptr(), data.len()) }
    }

    pub fn prefetch_bytecode(data: &[u8]) {
        unsafe { hermes__PrefetchHermesBytecode(data.as_ptr(), data.len()) }
    }

    pub fn enable_sampling_profiler() {
        unsafe { hermes__EnableSamplingProfiler() }
    }

    pub fn disable_sampling_profiler() {
        unsafe { hermes__DisableSamplingProfiler() }
    }

    pub fn dump_sampled_trace_to_file(filename: &str) {
        let c_str = std::ffi::CString::new(filename).expect("invalid filename");
        unsafe { hermes__DumpSampledTraceToFile(c_str.as_ptr()) }
    }

    pub unsafe fn set_fatal_handler(handler: HermesFatalHandler) {
        unsafe { hermes__SetFatalHandler(handler) }
    }

    pub fn get_bytecode_epilogue(data: &[u8]) -> Option<&[u8]> {
        let mut epilogue_len: usize = 0;
        let ptr =
            unsafe { hermes__GetBytecodeEpilogue(data.as_ptr(), data.len(), &mut epilogue_len) };
        if ptr.is_null() || epilogue_len == 0 {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(ptr, epilogue_len) })
        }
    }

    pub fn is_code_coverage_profiler_enabled() -> bool {
        unsafe { hermes__IsCodeCoverageProfilerEnabled() }
    }

    pub fn enable_code_coverage_profiler() {
        unsafe { hermes__EnableCodeCoverageProfiler() }
    }

    pub fn disable_code_coverage_profiler() {
        unsafe { hermes__DisableCodeCoverageProfiler() }
    }

    pub unsafe fn borrow_raw(ptr: *mut HermesRt) -> std::mem::ManuallyDrop<Runtime> {
        std::mem::ManuallyDrop::new(Runtime {
            raw: ptr,
            _not_send_sync: PhantomData,
        })
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { hermes__Runtime__Delete(self.raw) }
    }
}
