use rquickjs::{Context, Ctx, Exception, FromJs, Object, Promise, Runtime, convert::Coerced};
use serde::Deserialize;

use crate::error::{Result, RomError};

pub struct RomCore {
    runtime: Runtime,
    context: Context,
}

impl RomCore {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        Ok(Self { runtime, context })
    }

    pub fn with_context<T, F>(&self, callback: F) -> Result<T>
    where
        F: for<'js> FnOnce(Ctx<'js>) -> rquickjs::Result<T>,
    {
        let _keepalive = &self.runtime;
        Ok(self.context.with(callback)?)
    }

    fn with_captured_exception<T, F>(&self, callback: F) -> Result<T>
    where
        F: for<'js> FnOnce(Ctx<'js>) -> rquickjs::Result<T>,
    {
        let _keepalive = &self.runtime;
        self.context.with(|ctx| {
            Ok::<std::result::Result<T, RomError>, rquickjs::Error>(match callback(ctx.clone()) {
                Ok(value) => Ok(value),
                Err(error) => Err(capture_quickjs_error(&ctx, error)),
            })
        })?
    }

    pub fn eval<T>(&self, script: &str) -> Result<T>
    where
        for<'js> T: FromJs<'js>,
    {
        self.with_captured_exception(|ctx| ctx.eval(script))
    }

    pub fn eval_as_string(&self, script: &str) -> Result<String> {
        let encoded_script = serde_json::to_string(script)?;
        let wrapped = format!(
            r#"
            (() => {{
                try {{
                    const __gom_value = (0, eval)({encoded_script});
                    if (typeof globalThis.__rom_expose_webpack_require === "function") {{
                        try {{
                            globalThis.__rom_expose_webpack_require();
                        }} catch (_webpackExposeError) {{}}
                    }}
                    let __gom_result;
                    if (__gom_value === undefined) {{
                        __gom_result = "undefined";
                    }} else if (typeof __gom_value === "string") {{
                        __gom_result = __gom_value;
                    }} else {{
                        try {{
                            const __gom_json = JSON.stringify(__gom_value);
                            __gom_result = __gom_json === undefined ? String(__gom_value) : __gom_json;
                        }} catch (_error) {{
                            __gom_result = String(__gom_value);
                        }}
                    }}
                    return JSON.stringify({{ ok: true, value: __gom_result }});
                }} catch (__gom_error) {{
                    return JSON.stringify({{
                        ok: false,
                        error: String(__gom_error),
                        stack: __gom_error && typeof __gom_error === "object" ? (__gom_error.stack ?? null) : null,
                    }});
                }}
            }})()
            "#
        );

        self.extract_eval_string_result(self.eval(&wrapped)?)
    }

    pub fn eval_async<T>(&self, script: &str) -> Result<T>
    where
        for<'js> T: FromJs<'js>,
    {
        self.with_captured_exception(|ctx| {
            let promise: Promise<'_> = ctx.eval(script)?;
            promise.finish()
        })
    }

    pub fn eval_async_as_string(&self, script: &str) -> Result<String> {
        let encoded_script = serde_json::to_string(script)?;
        let wrapped = format!(
            r#"
            (async () => {{
                try {{
                    const __rom_value = await (0, eval)({encoded_script});
                    if (typeof globalThis.__rom_expose_webpack_require === "function") {{
                        try {{
                            globalThis.__rom_expose_webpack_require();
                        }} catch (_webpackExposeError) {{}}
                    }}
                    let __rom_result;
                    if (__rom_value === undefined) {{
                        __rom_result = "undefined";
                    }} else if (typeof __rom_value === "string") {{
                        __rom_result = __rom_value;
                    }} else {{
                        try {{
                            const __rom_json = JSON.stringify(__rom_value);
                            __rom_result = __rom_json === undefined ? String(__rom_value) : __rom_json;
                        }} catch (_error) {{
                            __rom_result = String(__rom_value);
                        }}
                    }}
                    return JSON.stringify({{ ok: true, value: __rom_result }});
                }} catch (__rom_error) {{
                    return JSON.stringify({{
                        ok: false,
                        error: String(__rom_error),
                        stack: __rom_error && typeof __rom_error === "object" ? (__rom_error.stack ?? null) : null,
                    }});
                }}
            }})()
            "#
        );

        self.extract_eval_string_result(self.eval_async(&wrapped)?)
    }

    fn extract_eval_string_result(&self, serialized: String) -> Result<String> {
        let outcome: EvalStringEnvelope = serde_json::from_str(&serialized)?;
        if outcome.ok {
            return Ok(outcome.value.unwrap_or_else(|| "undefined".to_owned()));
        }

        Err(RomError::QuickJsException(combine_exception_detail(
            outcome.error,
            outcome.stack,
        )))
    }
}

#[derive(Deserialize)]
struct EvalStringEnvelope {
    ok: bool,
    value: Option<String>,
    error: Option<String>,
    stack: Option<String>,
}

fn combine_exception_detail(error: Option<String>, stack: Option<String>) -> String {
    match (error, stack) {
        (Some(error), Some(stack)) if stack.contains(&error) => stack,
        (Some(error), Some(stack)) if stack.trim().is_empty() => error,
        (Some(error), Some(stack)) => format!("{error}\n{stack}"),
        (Some(error), None) => error,
        (None, Some(stack)) => stack,
        (None, None) => "JavaScript exception".to_owned(),
    }
}

fn capture_quickjs_error(ctx: &Ctx<'_>, error: rquickjs::Error) -> RomError {
    if error.is_exception() {
        return RomError::QuickJsException(describe_exception(ctx));
    }

    RomError::QuickJs(error)
}

fn describe_exception(ctx: &Ctx<'_>) -> String {
    let exception_value = ctx.catch();

    if let Ok(object) = Object::from_value(exception_value.clone())
        && let Some(exception) = Exception::from_object(object)
    {
        if let Some(stack) = exception.stack()
            && !stack.trim().is_empty()
        {
            return stack;
        }

        if let Some(message) = exception.message()
            && !message.trim().is_empty()
        {
            return message;
        }
    }

    Coerced::<String>::from_js(ctx, exception_value)
        .map(|value| value.0)
        .unwrap_or_else(|_| "JavaScript exception".to_owned())
}
