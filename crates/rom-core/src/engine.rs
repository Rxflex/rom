use rquickjs::{Context, Ctx, FromJs, Promise, Runtime};

use crate::error::Result;

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

    pub fn eval<T>(&self, script: &str) -> Result<T>
    where
        for<'js> T: FromJs<'js>,
    {
        self.with_context(|ctx| ctx.eval(script))
    }

    pub fn eval_as_string(&self, script: &str) -> Result<String> {
        let encoded_script = serde_json::to_string(script)?;
        let wrapped = format!(
            r#"
            (() => {{
                const __gom_value = (0, eval)({encoded_script});
                if (__gom_value === undefined) {{
                    return "undefined";
                }}
                if (typeof __gom_value === "string") {{
                    return __gom_value;
                }}
                try {{
                    const __gom_json = JSON.stringify(__gom_value);
                    return __gom_json === undefined ? String(__gom_value) : __gom_json;
                }} catch (_error) {{
                    return String(__gom_value);
                }}
            }})()
            "#
        );

        self.eval(&wrapped)
    }

    pub fn eval_async<T>(&self, script: &str) -> Result<T>
    where
        for<'js> T: FromJs<'js>,
    {
        self.with_context(|ctx| {
            let promise: Promise<'_> = ctx.eval(script)?;
            promise.finish()
        })
    }

    pub fn eval_async_as_string(&self, script: &str) -> Result<String> {
        let encoded_script = serde_json::to_string(script)?;
        let wrapped = format!(
            r#"
            (async () => {{
                const __rom_value = await (0, eval)({encoded_script});
                if (__rom_value === undefined) {{
                    return "undefined";
                }}
                if (typeof __rom_value === "string") {{
                    return __rom_value;
                }}
                try {{
                    const __rom_json = JSON.stringify(__rom_value);
                    return __rom_json === undefined ? String(__rom_value) : __rom_json;
                }} catch (_error) {{
                    return String(__rom_value);
                }}
            }})()
            "#
        );

        self.eval_async(&wrapped)
    }
}
