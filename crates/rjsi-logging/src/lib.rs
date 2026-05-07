// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rjsi_core::{Args, Context, Engine, Function, Object, Result, Value};

pub const NEWLINE: char = '\n';
pub const CARRIAGE_RETURN: char = '\r';
const SPACING: char = ' ';
#[allow(dead_code)]
const CIRCULAR: &str = "[Circular]";
pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

#[allow(dead_code)]
const MAX_INDENTATION_LEVEL: usize = 4;
#[allow(dead_code)]
const MAX_EXPANSION_DEPTH: usize = 4;
#[allow(dead_code)]
const INDENTATION_LOOKUP: [&str; MAX_INDENTATION_LEVEL + 1] =
    ["", "  ", "    ", "        ", "                "];

macro_rules! ascii_colors {
        ( $( $name:ident => $value:expr ),* ) => {
            #[derive(Debug, Clone, Copy)]
            pub enum Color {
                $(
                    $name,
                )*
            }

            impl AsRef<str> for Color {
                fn as_ref(&self) -> &str {
                    match self {
                        $(
                            Color::$name => concat!("\x1b[", stringify!($value), "m"),
                        )*
                    }
                }
            }
        }
    }

ascii_colors!(
    RESET => 0,
    BOLD => 1,
    BLACK => 30,
    RED => 31,
    GREEN => 32,
    YELLOW => 33,
    BLUE => 34,
    MAGENTA => 35,
    CYAN => 36,
    WHITE => 37
);

impl Color {
    #[inline(always)]
    fn push(self, value: &mut String) {
        value.push_str(self.as_ref())
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn reset(value: &mut String) {
        value.push_str(Color::RESET.as_ref())
    }
}

#[derive(Clone)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 4,
    Error = 8,
    Fatal = 16,
}

trait PushByte {
    fn push_byte(&mut self, byte: u8);
}

impl PushByte for String {
    fn push_byte(&mut self, byte: u8) {
        unsafe { self.as_mut_vec() }.push(byte);
    }
}

impl LogLevel {
    #[allow(clippy::inherent_to_string)]
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        match self {
            LogLevel::Trace => String::from("TRACE"),
            LogLevel::Debug => String::from("DEBUG"),
            LogLevel::Info => String::from("INFO"),
            LogLevel::Warn => String::from("WARN"),
            LogLevel::Error => String::from("ERROR"),
            LogLevel::Fatal => String::from("FATAL"),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            "FATAL" => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
}

pub struct FormatOptions<'cx, E: Engine> {
    #[allow(dead_code)]
    newline: bool,
    #[allow(dead_code)]
    get_own_property_desc_fn: Function<'cx, E>,
    #[allow(dead_code)]
    object_prototype: Object<'cx, E>,
    color: bool,
    number_function: Function<'cx, E>,
    parse_float: Function<'cx, E>,
    parse_int: Function<'cx, E>,
}

impl<'cx, E: Engine> FormatOptions<'cx, E> {
    pub fn new(ctx: &mut Context<'cx, E>, color: bool, newline: bool) -> Result<Self> {
        let globals = ctx.globals();
        let number_function = globals.get(ctx, "Number")?.try_as_function()?;
        let parse_float = globals.get(ctx, "parseFloat")?.try_as_function()?;
        let parse_int = globals.get(ctx, "parseInt")?.try_as_function()?;
        let object_prototype = ctx.eval("Object.prototype")?.try_as_object()?;
        let get_own_property_desc_fn = ctx
            .eval("Object.getOwnPropertyDescriptor")?
            .try_as_function()?;

        Ok(Self {
            color,
            newline,
            get_own_property_desc_fn,
            object_prototype,
            number_function,
            parse_float,
            parse_int,
        })
    }
}

fn write_percent_format<'a, E: Engine>(
    result: &mut String,
    ctx: &mut Context<'a, E>,
    args: &Args<'a, E>,
    options: &mut FormatOptions<'a, E>,
    bytes: &[u8],
) -> Result<usize> {
    let len = bytes.len();
    let size = args.len();
    let mut i = 0usize;
    let mut arg_idx = 1usize;

    while i < len {
        let byte = bytes[i];
        if byte == b'%' && i + 1 < len {
            let next_byte = bytes[i + 1];
            i += 1;
            if arg_idx < size {
                i += 1;
                let next_val = args.get(arg_idx).expect("arg_idx < size");
                arg_idx += 1;

                match next_byte {
                    b's' => {
                        let str = next_val.to_string(ctx).unwrap_or_default();
                        result.push_str(str.as_str());
                        continue;
                    }
                    b'd' => {
                        let undefined = ctx.undefined();
                        let value = options.number_function.call(ctx, undefined, &[next_val])?;
                        options.color = false;
                        format_raw(result, ctx, value, options)?;
                        continue;
                    }
                    b'i' => {
                        let undefined = ctx.undefined();
                        let value = options.parse_int.call(ctx, undefined, &[next_val])?;
                        options.color = false;
                        format_raw(result, ctx, value, options)?;
                        continue;
                    }
                    b'f' => {
                        let undefined = ctx.undefined();
                        let value = options.parse_float.call(ctx, undefined, &[next_val])?;
                        options.color = false;
                        format_raw(result, ctx, value, options)?;
                        continue;
                    }
                    b'j' => {
                        // TODO: Implement JSON stringification
                        continue;
                    }
                    b'O' => {
                        // TODO: Implement object formatting
                        options.color = false;
                        format_raw(result, ctx, next_val, options)?;
                        continue;
                    }
                    b'o' => {
                        options.color = false;
                        format_raw(result, ctx, next_val, options)?;
                        continue;
                    }
                    b'c' => {
                        // TODO: Implement color formatting
                        continue;
                    }
                    b'%' => {
                        result.push_byte(byte);
                        continue;
                    }
                    _ => {
                        result.push_byte(byte);
                        result.push_byte(next_byte);
                        continue;
                    }
                }
            }
            result.push_byte(byte);
            result.push_byte(next_byte);
        } else {
            result.push_byte(byte);
        }

        i += 1;
    }

    Ok(arg_idx)
}

pub fn build_formatted_string<'a, E: Engine>(
    result: &mut String,
    ctx: &mut Context<'a, E>,
    args: Args<'a, E>,
    options: &mut FormatOptions<'a, E>,
) -> Result<()> {
    let size = args.len();
    if size == 0 {
        return Ok(());
    }

    let consumed = if size > 1 {
        let first = args.get(0).expect("len > 0");
        if let Some(s) = first.to_string(ctx).ok() {
            if s.find('%').is_none() {
                format_raw_string(result, s, options);
                1
            } else {
                write_percent_format(result, ctx, &args, options, s.as_bytes())?
            }
        } else {
            format_raw(result, ctx, first, options)?;
            1
        }
    } else {
        let v = args.get(0).expect("len > 0");
        format_raw(result, ctx, v, options)?;
        return Ok(());
    };

    for raw in args.rest_from(consumed).iter() {
        result.push(SPACING);
        format_raw(result, ctx, Value::new(raw), options)?;
    }

    Ok(())
}

#[inline(always)]
fn format_raw<'a, E: Engine>(
    result: &mut String,
    ctx: &mut Context<'a, E>,
    value: Value<'a, E>,
    _options: &FormatOptions<'a, E>,
) -> Result<()> {
    let s = value.to_string(ctx)?;
    result.push_str(&s);
    Ok(())
}

fn format_raw_string<'a, E: Engine>(
    result: &mut String,
    value: String,
    options: &mut FormatOptions<'a, E>,
) {
    format_raw_string_inner(result, value, false, options.color);
}

fn format_raw_string_inner(result: &mut String, value: String, quoted: bool, color_enabled: bool) {
    if quoted {
        if color_enabled {
            Color::GREEN.push(result);
        }
        result.push('\'');
    }
    result.push_str(&value);
    if quoted {
        result.push('\'');
    }
}
