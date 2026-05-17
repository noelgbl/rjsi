// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::{IsTerminal, Write, stderr, stdout};

use rjsi_core::{Args, Context, Engine, Result, Value};
use rjsi_logging::{FormatOptions, NEWLINE, build_formatted_string};

pub fn log_fatal<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    write_log(stderr(), cx, args)?;
    Ok(cx.undefined())
}

pub fn log_error<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    write_log(stderr(), cx, args)?;
    Ok(cx.undefined())
}

fn log_warn<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    write_log(stderr(), cx, args)?;
    Ok(cx.undefined())
}

fn log_debug<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    write_log(stdout(), cx, args)?;
    Ok(cx.undefined())
}

fn log_trace<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    write_log(stdout(), cx, args)?;
    Ok(cx.undefined())
}

fn log_assert<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    let expression = args.get(0).and_then(|v| v.as_bool()).unwrap_or(true);

    if expression {
        write_log(stderr(), cx, args)?;
    }

    Ok(cx.undefined())
}

fn log<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    write_log(stdout(), cx, args)?;
    Ok(cx.undefined())
}

fn clear<'rt, E: Engine>(
    cx: &mut Context<'rt, E>,
    _this: Value<'rt, E>,
    _args: Args<'rt, E>,
) -> Result<Value<'rt, E>> {
    let _ = stdout().write_all(b"\x1b[1;1H\x1b[0J");
    Ok(cx.undefined())
}

fn write_log<'rt, E: Engine, T>(
    mut output: T,
    ctx: &mut Context<'rt, E>,
    args: Args<'rt, E>,
) -> Result<()>
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

pub fn init<'cx, E: Engine>(ctx: &mut Context<'cx, E>) -> Result<()> {
    let globals = ctx.globals();

    let console = ctx.new_object()?;

    let assert = ctx.raw_function("assert", log_assert)?.into_value();
    console.set(ctx, "assert", assert)?;
    let clear = ctx.raw_function("clear", clear)?.into_value();
    console.set(ctx, "clear", clear)?;
    let debug = ctx.raw_function("debug", log_debug)?.into_value();
    console.set(ctx, "debug", debug)?;
    let error = ctx.raw_function("error", log_error)?.into_value();
    console.set(ctx, "error", error)?;
    let info = ctx.raw_function("info", log)?.into_value();
    console.set(ctx, "info", info)?;
    let log = ctx.raw_function("log", log)?.into_value();
    console.set(ctx, "log", log)?;
    let trace = ctx.raw_function("trace", log_trace)?.into_value();
    console.set(ctx, "trace", trace)?;
    let warn = ctx.raw_function("warn", log_warn)?.into_value();
    console.set(ctx, "warn", warn)?;

    globals.set(ctx, "console", console.into_value())?;

    Ok(())
}
