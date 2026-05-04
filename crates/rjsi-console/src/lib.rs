// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::{IsTerminal, Write, stderr, stdout};

use rjsi_core::{Args, CallbackCx, Context, Engine, JsResult, Value};
use rjsi_logging::{FormatOptions, NEWLINE, build_formatted_string};

pub fn log_fatal<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    write_log(stderr(), cx.cx(), args)?;
    Ok(cx.cx().undefined())
}

pub fn log_error<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    write_log(stderr(), cx.cx(), args)?;
    Ok(cx.cx().undefined())
}

fn log_warn<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    write_log(stderr(), cx.cx(), args)?;
    Ok(cx.cx().undefined())
}

fn log_debug<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    write_log(stdout(), cx.cx(), args)?;
    Ok(cx.cx().undefined())
}

fn log_trace<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    write_log(stdout(), cx.cx(), args)?;
    Ok(cx.cx().undefined())
}

fn log_assert<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    let expression = args.get(0).and_then(|v| v.to_bool()).unwrap_or(true);

    if expression {
        write_log(stderr(), cx.cx(), args)?;
    }

    Ok(cx.cx().undefined())
}

fn log<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    write_log(stdout(), cx.cx(), args)?;
    Ok(cx.cx().undefined())
}

fn clear<'cx, 'rt, E: Engine>(
    cx: &mut CallbackCx<'cx, 'rt, E>,
    _this: Value<'rt, E>,
    _args: Args<'rt, E>,
) -> JsResult<Value<'rt, E>> {
    let _ = stdout().write_all(b"\x1b[1;1H\x1b[0J");
    Ok(cx.cx().undefined())
}

fn write_log<'rt, E: Engine, T>(
    mut output: T,
    ctx: &mut Context<'rt, E>,
    args: Args<'rt, E>,
) -> JsResult<()>
where
    T: Write + IsTerminal,
{
    let is_tty = output.is_terminal();
    let mut result = String::new();

    let mut options = FormatOptions::new(ctx, is_tty, true)?;
    build_formatted_string(&mut result, ctx, args, &mut options)?;

    result.push(NEWLINE);

    // we don't care if output is interrupted
    let _ = output.write_all(result.as_bytes());

    Ok(())
}

pub fn init<'cx, E: Engine>(ctx: &mut Context<'cx, E>) -> JsResult<()> {
    let globals = ctx.globals();

    let console = ctx.new_object()?;

    let assert = ctx.function("assert", log_assert)?.into_value();
    console.set(ctx, "assert", assert)?;
    let clear = ctx.function("clear", clear)?.into_value();
    console.set(ctx, "clear", clear)?;
    let debug = ctx.function("debug", log_debug)?.into_value();
    console.set(ctx, "debug", debug)?;
    let error = ctx.function("error", log_error)?.into_value();
    console.set(ctx, "error", error)?;
    let info = ctx.function("info", log)?.into_value();
    console.set(ctx, "info", info)?;
    let log = ctx.function("log", log)?.into_value();
    console.set(ctx, "log", log)?;
    let trace = ctx.function("trace", log_trace)?.into_value();
    console.set(ctx, "trace", trace)?;
    let warn = ctx.function("warn", log_warn)?.into_value();
    console.set(ctx, "warn", warn)?;

    globals.set(ctx, "console", console.into_value())?;

    Ok(())
}
