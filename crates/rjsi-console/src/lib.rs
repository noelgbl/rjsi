use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::marker::PhantomData;
use std::time::Instant;

use rjsi_core::{
    HostError, HostFunction, JsEngine, JsResult, JsRuntime, JsScope, JsValueType, ParamsAccessor
};

/// Installs the generic `console` and invokes `console.log("hello")`. Use with
/// every [`JsRuntime`](rjsi_core::JsRuntime) in integration tests.
pub fn smoke_install_and_log<R: JsRuntime>(runtime: &R) -> JsResult<()> {
    fn ok<T, E>(r: Result<T, E>, what: &'static str) -> T {
        r.unwrap_or_else(|_| panic!("rjsi_console::smoke_install_and_log: {what}"))
    }
    runtime.with_scope(|scope| {
        init(scope)?;
        let console_key = scope.static_property_key("console");
        let log_key = scope.static_property_key("log");
        let global = scope.global();
        let console = ok(
            scope.get_property(&global, &console_key),
            "get global.console",
        )
        .unwrap();
        let log = ok(scope.get_property(&console, &log_key), "get console.log").unwrap();
        let arg = scope.string("hello");
        ok(
            scope.call_function(&log, Some(&console), &[arg]),
            "call console.log",
        );
        Ok(())
    })
}

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

pub fn set_trace_context<E: JsEngine>(trace_context: ConsoleTraceContext) {
    let _ = PhantomData::<E>;
    TRACE_CONTEXT.with(|trace| *trace.borrow_mut() = Some(trace_context));
}

pub fn clear_trace_context<E: JsEngine>() {
    let _ = PhantomData::<E>;
    TRACE_CONTEXT.with(|trace| *trace.borrow_mut() = None);
}

pub fn trace_context<E: JsEngine>() -> Option<ConsoleTraceContext> {
    let _ = PhantomData::<E>;
    TRACE_CONTEXT.with(|trace| trace.borrow().clone())
}

pub fn init<'js, S: JsScope<'js>>(scope: &mut S) -> JsResult<()> {
    let console = scope.object();
    for (name, kind) in [
        ("clear", ConsoleMethodKind::Clear),
        ("log", ConsoleMethodKind::Log(LogLevel::Info)),
        ("error", ConsoleMethodKind::Log(LogLevel::Error)),
        ("warn", ConsoleMethodKind::Log(LogLevel::Warn)),
        ("info", ConsoleMethodKind::Log(LogLevel::Info)),
        ("debug", ConsoleMethodKind::Log(LogLevel::Debug)),
        ("assert", ConsoleMethodKind::Assert),
        ("dir", ConsoleMethodKind::Dir),
        ("trace", ConsoleMethodKind::Trace),
        ("time", ConsoleMethodKind::Time),
        ("timeLog", ConsoleMethodKind::TimeLog),
        ("timeEnd", ConsoleMethodKind::TimeEnd),
        ("count", ConsoleMethodKind::Count),
        ("countReset", ConsoleMethodKind::CountReset),
    ] {
        define_method(scope, &console, name, kind)?;
    }

    let global = scope.global();
    let key = scope.static_property_key("console");
    scope
        .set_property(&global, &key, &console)
        .map_err(|thrown| host_error_from_thrown(scope, &thrown, "failed to install console"))?;
    Ok(())
}

fn define_method<'js, S: JsScope<'js>>(
    scope: &mut S,
    console: &<S::Engine as JsEngine>::Value<'js>,
    name: &'static str,
    kind: ConsoleMethodKind,
) -> JsResult<()> {
    let function = scope
        .host_function(
            name,
            ConsoleMethod::<S::Engine> {
                kind,
                marker: PhantomData,
            },
        )
        .map_err(|thrown| {
            host_error_from_thrown(scope, &thrown, &format!("failed to create console.{name}"))
        })?;
    let key = scope.static_property_key(name);
    scope
        .set_property(console, &key, &function)
        .map_err(|thrown| {
            host_error_from_thrown(scope, &thrown, &format!("failed to set console.{name}"))
        })?;
    Ok(())
}

#[derive(Clone, Copy)]
enum ConsoleMethodKind {
    Log(LogLevel),
    Assert,
    Clear,
    Dir,
    Trace,
    Time,
    TimeLog,
    TimeEnd,
    Count,
    CountReset,
}

struct ConsoleMethod<E: JsEngine> {
    kind: ConsoleMethodKind,
    marker: PhantomData<fn() -> E>,
}

impl<E: JsEngine> HostFunction<E> for ConsoleMethod<E> {
    fn call<'a, 'js>(&mut self, params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
    where
        'js: 'a,
    {
        match self.kind {
            ConsoleMethodKind::Log(level) => log(level, params),
            ConsoleMethodKind::Assert => assert(params),
            ConsoleMethodKind::Clear => clear(params),
            ConsoleMethodKind::Dir => dir(params),
            ConsoleMethodKind::Trace => trace(params),
            ConsoleMethodKind::Time => time(params),
            ConsoleMethodKind::TimeLog => time_log(params),
            ConsoleMethodKind::TimeEnd => time_end(params),
            ConsoleMethodKind::Count => count(params),
            ConsoleMethodKind::CountReset => count_reset(params),
        }
    }
}

fn log<'a, 'js, E: JsEngine>(
    level: LogLevel,
    params: &mut ParamsAccessor<'a, 'js, E>,
) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let values = collect_args(params);
    let message = format_values(params.scope(), &values, false);
    write_console::<E>(level, &message);
    Ok(params.scope().undefined())
}

fn assert<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let condition = params.next_arg();
    if condition
        .as_ref()
        .is_some_and(|value| js_value_is_truthy(params.scope(), value))
    {
        return Ok(params.scope().undefined());
    }

    let values = collect_args(params);
    let message = if values.is_empty() {
        "Assertion failed".to_owned()
    } else {
        format!(
            "Assertion failed: {}",
            format_values(params.scope(), &values, false)
        )
    };
    write_console::<E>(LogLevel::Assert, &message);
    Ok(params.scope().undefined())
}

fn clear<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    if io::stdout().is_terminal() {
        print!("\x1B[2J\x1B[1;1H");
        let _ = io::stdout().flush();
    } else {
        println!();
    }
    Ok(params.scope().undefined())
}

fn dir<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let value = params.next_arg();
    let message = value
        .as_ref()
        .map(|value| format_value(params.scope(), value, true))
        .unwrap_or_else(|| "undefined".to_owned());
    write_console::<E>(LogLevel::Info, &message);
    Ok(params.scope().undefined())
}

fn trace<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let values = collect_args(params);
    let message = if values.is_empty() {
        "Trace".to_owned()
    } else {
        format!("Trace: {}", format_values(params.scope(), &values, false))
    };
    write_console::<E>(LogLevel::Trace, &message);
    Ok(params.scope().undefined())
}

fn time<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let label = next_label(params, "default");
    CONSOLE_STATE.with(|state| {
        state.timers.borrow_mut().insert(label, Instant::now());
    });
    Ok(params.scope().undefined())
}

fn time_log<'a, 'js, E: JsEngine>(
    params: &mut ParamsAccessor<'a, 'js, E>,
) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let label = next_label(params, "default");
    let started_at = CONSOLE_STATE.with(|state| state.timers.borrow().get(&label).copied());
    let Some(started_at) = started_at else {
        write_console::<E>(LogLevel::Warn, &format!("Timer '{label}' does not exist"));
        return Ok(params.scope().undefined());
    };

    let extras = collect_args(params);
    let mut message = format!("{label}: {}", format_elapsed_ms(started_at.elapsed()));
    if !extras.is_empty() {
        message.push(' ');
        message.push_str(&format_values(params.scope(), &extras, false));
    }
    write_console::<E>(LogLevel::Info, &message);
    Ok(params.scope().undefined())
}

fn time_end<'a, 'js, E: JsEngine>(
    params: &mut ParamsAccessor<'a, 'js, E>,
) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let label = next_label(params, "default");
    let started_at = CONSOLE_STATE.with(|state| state.timers.borrow_mut().remove(&label));
    match started_at {
        Some(started_at) => write_console::<E>(
            LogLevel::Info,
            &format!("{label}: {}", format_elapsed_ms(started_at.elapsed())),
        ),
        None => write_console::<E>(LogLevel::Warn, &format!("Timer '{label}' does not exist")),
    }
    Ok(params.scope().undefined())
}

fn count<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let label = next_label(params, "default");
    let next = CONSOLE_STATE.with(|state| {
        let mut counters = state.counters.borrow_mut();
        let count = counters.entry(label.clone()).or_insert(0);
        *count += 1;
        *count
    });
    write_console::<E>(LogLevel::Info, &format!("{label}: {next}"));
    Ok(params.scope().undefined())
}

fn count_reset<'a, 'js, E: JsEngine>(
    params: &mut ParamsAccessor<'a, 'js, E>,
) -> JsResult<E::Value<'js>>
where
    'js: 'a,
{
    let label = next_label(params, "default");
    let removed = CONSOLE_STATE.with(|state| state.counters.borrow_mut().remove(&label).is_some());
    if !removed {
        write_console::<E>(
            LogLevel::Warn,
            &format!("Count for '{label}' does not exist"),
        );
    }
    Ok(params.scope().undefined())
}

fn collect_args<'a, 'js, E: JsEngine>(params: &mut ParamsAccessor<'a, 'js, E>) -> Vec<E::Value<'js>>
where
    'js: 'a,
{
    let mut values = Vec::with_capacity(params.len());
    while let Some(value) = params.next_arg() {
        values.push(value);
    }
    values
}

fn next_label<'a, 'js, E: JsEngine>(
    params: &mut ParamsAccessor<'a, 'js, E>,
    default: &str,
) -> String
where
    'js: 'a,
{
    params
        .next_arg()
        .as_ref()
        .and_then(|value| params.scope().to_string(value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn format_values<'js, S: JsScope<'js>>(
    scope: &mut S,
    values: &[<S::Engine as JsEngine>::Value<'js>],
    quote_top_level_string: bool,
) -> String {
    let mut result = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            result.push(' ');
        }
        result.push_str(&format_value(scope, value, quote_top_level_string));
    }
    result
}

fn format_value<'js, S: JsScope<'js>>(
    scope: &mut S,
    value: &<S::Engine as JsEngine>::Value<'js>,
    quote_top_level_string: bool,
) -> String {
    match scope.value_type(value) {
        JsValueType::Undefined => "undefined".to_owned(),
        JsValueType::Null => "null".to_owned(),
        JsValueType::Boolean => scope
            .to_boolean(value)
            .map(|value| if value { "true" } else { "false" }.to_owned())
            .unwrap_or_else(|| "[Boolean]".to_owned()),
        JsValueType::Number | JsValueType::BigInt | JsValueType::Date => scope
            .to_string(value)
            .unwrap_or_else(|| format!("[{}]", scope.value_type(value))),
        JsValueType::String => {
            let text = scope.to_string(value).unwrap_or_default();
            if quote_top_level_string {
                format!("\"{}\"", escape_string(&text))
            } else {
                text
            }
        }
        JsValueType::Function => function_name(scope, value),
        JsValueType::Array => scope
            .to_string(value)
            .map(|value| {
                if value.is_empty() {
                    "[]".to_owned()
                } else {
                    format!("[ {value} ]")
                }
            })
            .unwrap_or_else(|| "[Array]".to_owned()),
        JsValueType::ArrayBuffer => scope
            .to_string(value)
            .unwrap_or_else(|| "ArrayBuffer".to_owned()),
        JsValueType::Object => scope
            .to_string(value)
            .filter(|text| text != "[object Object]")
            .unwrap_or_else(|| "{...}".to_owned()),
        JsValueType::Error | JsValueType::Exception => {
            scope.to_string(value).unwrap_or_else(|| "Error".to_owned())
        }
        JsValueType::Symbol | JsValueType::Promise | JsValueType::Unknown => scope
            .to_string(value)
            .unwrap_or_else(|| format!("[{}]", scope.value_type(value))),
    }
}

fn function_name<'js, S: JsScope<'js>>(
    scope: &mut S,
    value: &<S::Engine as JsEngine>::Value<'js>,
) -> String {
    let key = scope.static_property_key("name");
    match scope.get_property(value, &key) {
        Ok(Some(name)) => scope
            .to_string(&name)
            .filter(|name| !name.is_empty())
            .map(|name| format!("[Function: {name}]"))
            .unwrap_or_else(|| "[Function]".to_owned()),
        _ => "[Function]".to_owned(),
    }
}

fn js_value_is_truthy<'js, S: JsScope<'js>>(
    scope: &mut S,
    value: &<S::Engine as JsEngine>::Value<'js>,
) -> bool {
    match scope.value_type(value) {
        JsValueType::Undefined | JsValueType::Null => false,
        JsValueType::Boolean => scope.to_boolean(value).unwrap_or(false),
        JsValueType::Number => scope
            .to_number(value)
            .map(|number| number != 0.0 && !number.is_nan())
            .unwrap_or(false),
        JsValueType::String => scope
            .to_string(value)
            .map(|text| !text.is_empty())
            .unwrap_or(false),
        JsValueType::BigInt => scope
            .to_string(value)
            .map(|text| text != "0" && text != "0n")
            .unwrap_or(true),
        JsValueType::Unknown => false,
        _ => true,
    }
}

fn write_console<E: JsEngine>(level: LogLevel, message: &str) {
    if tracing::dispatcher::has_been_set() {
        emit_console_trace::<E>(level, message);
        return;
    }

    match level {
        LogLevel::Verbose | LogLevel::Info => println!("{message}"),
        LogLevel::Debug => println!("DEBUG: {message}"),
        LogLevel::Error => eprintln!("ERROR: {message}"),
        LogLevel::Warn => eprintln!("WARN: {message}"),
        LogLevel::Trace | LogLevel::Assert => eprintln!("{message}"),
    }
}

fn emit_console_trace<E: JsEngine>(level: LogLevel, message: &str) {
    macro_rules! emit_at_level {
        ($level:expr) => {
            match trace_context::<E>() {
                Some(trace) => match (trace.namespace.as_deref(), trace.scope.as_deref()) {
                    (Some(namespace), Some(scope)) => tracing::event!(
                        target: "rjsi.js.console",
                        $level,
                        namespace,
                        scope,
                        message = message
                    ),
                    (Some(namespace), None) => tracing::event!(
                        target: "rjsi.js.console",
                        $level,
                        namespace,
                        message = message
                    ),
                    (None, Some(scope)) => tracing::event!(
                        target: "rjsi.js.console",
                        $level,
                        scope,
                        message = message
                    ),
                    (None, None) => tracing::event!(
                        target: "rjsi.js.console",
                        $level,
                        message = message
                    ),
                },
                None => tracing::event!(
                    target: "rjsi.js.console",
                    $level,
                    message = message
                ),
            }
        };
    }

    match level {
        LogLevel::Verbose | LogLevel::Info => {
            emit_at_level!(tracing::Level::INFO)
        }
        LogLevel::Debug => emit_at_level!(tracing::Level::DEBUG),
        LogLevel::Error | LogLevel::Trace | LogLevel::Assert => {
            emit_at_level!(tracing::Level::ERROR);
        }
        LogLevel::Warn => emit_at_level!(tracing::Level::WARN),
    }
}

fn host_error_from_thrown<'js, S: JsScope<'js>>(
    scope: &mut S,
    thrown: &<S::Engine as JsEngine>::Value<'js>,
    fallback: &str,
) -> rjsi_core::RjsiJSError {
    HostError::new(
        rjsi_core::error::E_ERROR,
        scope
            .to_string(thrown)
            .unwrap_or_else(|| fallback.to_owned()),
    )
    .into()
}

fn format_elapsed_ms(duration: std::time::Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1000.0)
}

fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            c if c.is_ascii_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}
