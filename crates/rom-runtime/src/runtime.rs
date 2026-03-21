use rom_core::RomCore;
use rom_webapi::install_browser_api;

use crate::{config::RuntimeConfig, error::Result};

pub struct RomRuntime {
    core: RomCore,
}

impl RomRuntime {
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let core = RomCore::new()?;
        let web_config = config.to_web_config()?;

        core.with_context(|ctx| install_browser_api(ctx, &web_config))?;

        Ok(Self { core })
    }

    pub fn eval<T>(&self, script: &str) -> Result<T>
    where
        for<'js> T: rquickjs::FromJs<'js>,
    {
        Ok(self.core.eval(script)?)
    }

    pub fn eval_as_string(&self, script: &str) -> Result<String> {
        Ok(self.core.eval_as_string(script)?)
    }

    pub fn eval_async<T>(&self, script: &str) -> Result<T>
    where
        for<'js> T: rquickjs::FromJs<'js>,
    {
        Ok(self.core.eval_async(script)?)
    }

    pub fn eval_async_as_string(&self, script: &str) -> Result<String> {
        Ok(self.core.eval_async_as_string(script)?)
    }

    pub fn export_cookie_store(&self) -> Result<String> {
        Ok(self.core.eval_as_string("__rom_export_cookie_store()")?)
    }
}
