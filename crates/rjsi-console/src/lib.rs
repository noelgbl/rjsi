use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::time::Instant;

use rjsi_core::{ContextLike, Runtime, ScopeLike, ValueLike};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleTraceContext {
    pub namespace: Option<String>,
    pub scope: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Verbose,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Assert,
}

#[derive(Default)]
struct ConsoleRuntimeState {
    timers: RefCell<HashMap<String, Instant>>,
    counters: RefCell<HashMap<String, usize>>,
}

thread_local! {
    static CONSOLE_STATE: ConsoleRuntimeState = ConsoleRuntimeState::default();
    static TRACE_CONTEXT: RefCell<Option<ConsoleTraceContext>> = const { RefCell::new(None) };
}

pub fn set_trace_context<R: Runtime>(trace_context: ConsoleTraceContext) {
    let _ = std::marker::PhantomData::<R>;
    TRACE_CONTEXT.with(|trace| *trace.borrow_mut() = Some(trace_context));
}

pub fn clear_trace_context<R: Runtime>() {
    let _ = std::marker::PhantomData::<R>;
    TRACE_CONTEXT.with(|trace| *trace.borrow_mut() = None);
}

pub fn trace_context<R: Runtime>() -> Option<ConsoleTraceContext> {
    let _ = std::marker::PhantomData::<R>;
    TRACE_CONTEXT.with(|trace| trace.borrow().clone())
}

pub fn smoke_install_and_log<R>(runtime: &R::Context) -> Result<(), R::Error>
where
    R: Runtime,
    R::Context: ContextLike<R>,
{
    runtime.with_scope(|scope| {
        init::<R>(scope)?;
        scope.eval("console.log('hello')")?;
        Ok(())
    })
}

pub fn init<'s, 'p, R>(scope: &mut R::Scope<'s, 'p>) -> Result<(), R::Error>
where
    R: Runtime,
{
    let console = scope.object();
    define_method::<R>(scope, &console, "log", LogLevel::Info)?;
    define_method::<R>(scope, &console, "error", LogLevel::Error)?;
    define_method::<R>(scope, &console, "warn", LogLevel::Warn)?;
    define_method::<R>(scope, &console, "info", LogLevel::Info)?;
    define_method::<R>(scope, &console, "debug", LogLevel::Debug)?;
    define_method::<R>(scope, &console, "trace", LogLevel::Trace)?;
    define_method::<R>(scope, &console, "assert", LogLevel::Assert)?;
    let global = scope.global();
    global.set(scope, "console", console);
    Ok(())
}

fn define_method<'s, 'p, R>(
    scope: &mut R::Scope<'s, 'p>,
    console: &R::Value<'s>,
    name: &'static str,
    level: LogLevel,
) -> Result<(), R::Error>
where
    R: Runtime,
{
    let function = scope.function(move |scope, args| {
        console_call::<R>(scope, level, args)
    })?;
    console.set(scope, name, function);
    Ok(())
}

fn console_call<'s, R>(
    scope: &mut R::Scope<'s, 's>,
    level: LogLevel,
    args: rjsi_core::Args<'s, R>,
) -> Result<R::Value<'s>, R::Error>
where
    R: Runtime,
{
    match level {
        LogLevel::Assert => {
            let condition = args
                .value(0)
                .map(|v| v.as_bool(scope).unwrap_or(false))
                .unwrap_or(false);
            if condition {
                return Ok(scope.undefined());
            }
            let rendered = render_values::<R>(scope, &args.as_slice()[1..]);
            let message = if rendered.is_empty() {
                "Assertion failed".to_string()
            } else {
                format!("Assertion failed: {rendered}")
            };
            write_console::<R>(level, &message);
        }
        LogLevel::Trace => {
            let rendered = render_values::<R>(scope, args.as_slice());
            let message = if rendered.is_empty() {
                "trace".to_string()
            } else {
                rendered
            };
            write_console::<R>(level, &message);
        }
        _ => {
            let rendered = render_values::<R>(scope, args.as_slice());
            write_console::<R>(level, &rendered);
        }
    }
    Ok(scope.undefined())
}

fn render_values<'s, R>(scope: &mut R::Scope<'s, '_>, values: &[R::Value<'s>]) -> String
where
    R: Runtime,
{
    values
        .iter()
        .map(|value| render_value::<R>(scope, value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_value<'s, R>(scope: &mut R::Scope<'s, '_>, value: &R::Value<'s>) -> String
where
    R: Runtime,
{
    if value.is_string() {
        return value
            .with_str(scope, str::to_owned)
            .unwrap_or_else(|| "[string]".to_string());
    }
    if value.is_number() {
        return value
            .as_f64(scope)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "[number]".to_string());
    }
    if value.is_boolean() {
        return value.as_bool(scope).unwrap_or(false).to_string();
    }
    if value.is_null() {
        return "null".to_string();
    }
    if value.is_undefined() {
        return "undefined".to_string();
    }
    if value.is_function() {
        return "[Function]".to_string();
    }
    if value.is_array() {
        return "[Array]".to_string();
    }
    if value.is_object() {
        return "[Object]".to_string();
    }
    "[Value]".to_string()
}

fn write_console<R: Runtime>(level: LogLevel, message: &str) {
    let prefix = match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
        LogLevel::Assert => "assert",
        LogLevel::Verbose => "verbose",
    };

    if io::stdout().is_terminal() {
        println!("[{prefix}] {message}");
    } else {
        println!("{message}");
    }
    let _ = io::stdout().flush();

    TRACE_CONTEXT.with(|trace| {
        if let Some(trace) = trace.borrow().clone() {
            let ns = trace.namespace.unwrap_or_else(|| "console".to_string());
            let sc = trace.scope.unwrap_or_default();
            eprintln!("trace {ns} {sc} {prefix}: {message}");
        }
    });

    CONSOLE_STATE.with(|state| {
        let _ = &state.timers;
        let _ = &state.counters;
    });
}
