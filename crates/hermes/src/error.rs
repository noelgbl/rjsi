use std::fmt;

use hermes_sys::*;

use crate::value::is_pointer_kind;

unsafe extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

pub(crate) fn check_error(rt: *mut HermesRt) -> Result<()> {
    unsafe {
        if !hermes__Runtime__HasPendingError(rt) {
            return Ok(());
        }

        let c_msg = hermes__Runtime__GetAndClearErrorMessage(rt);

        let mut err_val = hermes__Runtime__GetAndClearError(rt);
        let js_msg = extract_error_message(rt, &err_val);

        if is_pointer_kind(err_val.kind) {
            hermes__Value__Release(&mut err_val);
        }

        if !c_msg.is_null() {
            let s = std::ffi::CStr::from_ptr(c_msg)
                .to_string_lossy()
                .into_owned();
            free(c_msg as *mut _);
            return Err(Error::JsException(s));
        }
        if !js_msg.is_empty() {
            return Err(Error::JsException(js_msg));
        }
        Err(Error::JsException("unknown error".into()))
    }
}

unsafe fn extract_error_message(rt: *mut HermesRt, val: &HermesValue) -> String {
    unsafe {
        use crate::string::pv_to_rust_string_lossy;

        match val.kind {
            HermesValueKind_String => pv_to_rust_string_lossy(rt, val.data.pointer),
            HermesValueKind_Object => {
                let key = b"message";
                let key_pv = hermes__String__CreateFromUtf8(rt, key.as_ptr(), key.len());
                let msg_val = hermes__Object__GetProperty__String(rt, val.data.pointer, key_pv);
                hermes__String__Release(key_pv);
                if msg_val.kind == HermesValueKind_String {
                    let s = pv_to_rust_string_lossy(rt, msg_val.data.pointer);
                    let mut mv = msg_val;
                    hermes__Value__Release(&mut mv);
                    s
                } else {
                    if is_pointer_kind(msg_val.kind) {
                        let mut mv = msg_val;
                        hermes__Value__Release(&mut mv);
                    }
                    String::new()
                }
            }
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    JsException(String),

    TypeError {
        expected: &'static str,
        got: &'static str,
    },

    RuntimeError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::JsException(msg) => write!(f, "JavaScript exception: {msg}"),
            Error::TypeError { expected, got } => {
                write!(f, "type error: expected {expected}, got {got}")
            }
            Error::RuntimeError(msg) => write!(f, "runtime error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
