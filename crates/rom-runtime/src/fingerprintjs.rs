use serde::{Deserialize, Serialize};

use crate::{Result, RomRuntime};

const FINGERPRINTJS_VERSION: &str = "5.1.0";
const FINGERPRINTJS_UMD_BUNDLE: &str = include_str!("../../../fixtures/package/dist/fp.umd.min.js");
const DEFAULT_FINGERPRINTJS_HARNESS_SNAPSHOT: &str =
    include_str!("../../../fixtures/fingerprintjs/rom-default-harness.json");
const BROWSER_CHROMIUM_HARNESS_SNAPSHOT: &str =
    include_str!("../../../fixtures/fingerprintjs/browser-chromium-harness.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintJsHarnessReport {
    pub ok: bool,
    pub version: Option<String>,
    pub visitor_id: Option<String>,
    pub confidence_score: Option<f64>,
    pub component_count: usize,
    pub error_component_count: usize,
    pub component_keys: Vec<String>,
    pub failed_components: Vec<String>,
    pub failed_component_errors: Vec<ComponentError>,
    pub top_level_error: Option<HarnessError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessError {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentError {
    pub key: String,
    pub name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintJsHarnessDiff {
    pub ok_changed: bool,
    pub version_changed: bool,
    pub visitor_id_changed: bool,
    pub confidence_score_changed: bool,
    pub component_count_changed: bool,
    pub error_component_count_changed: bool,
    pub missing_component_keys: Vec<String>,
    pub unexpected_component_keys: Vec<String>,
    pub missing_failed_components: Vec<String>,
    pub unexpected_failed_components: Vec<String>,
}

impl FingerprintJsHarnessDiff {
    pub fn is_empty(&self) -> bool {
        !self.ok_changed
            && !self.version_changed
            && !self.visitor_id_changed
            && !self.confidence_score_changed
            && !self.component_count_changed
            && !self.error_component_count_changed
            && self.missing_component_keys.is_empty()
            && self.unexpected_component_keys.is_empty()
            && self.missing_failed_components.is_empty()
            && self.unexpected_failed_components.is_empty()
    }

    pub fn without_identity(&self) -> Self {
        Self {
            visitor_id_changed: false,
            ..self.clone()
        }
    }
}

impl FingerprintJsHarnessReport {
    pub fn diff(&self, baseline: &Self) -> FingerprintJsHarnessDiff {
        FingerprintJsHarnessDiff {
            ok_changed: self.ok != baseline.ok,
            version_changed: self.version != baseline.version,
            visitor_id_changed: self.visitor_id != baseline.visitor_id,
            confidence_score_changed: self.confidence_score != baseline.confidence_score,
            component_count_changed: self.component_count != baseline.component_count,
            error_component_count_changed: self.error_component_count
                != baseline.error_component_count,
            missing_component_keys: baseline
                .component_keys
                .iter()
                .filter(|key| !self.component_keys.contains(*key))
                .cloned()
                .collect(),
            unexpected_component_keys: self
                .component_keys
                .iter()
                .filter(|key| !baseline.component_keys.contains(*key))
                .cloned()
                .collect(),
            missing_failed_components: baseline
                .failed_components
                .iter()
                .filter(|key| !self.failed_components.contains(*key))
                .cloned()
                .collect(),
            unexpected_failed_components: self
                .failed_components
                .iter()
                .filter(|key| !baseline.failed_components.contains(*key))
                .cloned()
                .collect(),
        }
    }
}

impl RomRuntime {
    pub fn run_fingerprintjs_harness(&self) -> Result<FingerprintJsHarnessReport> {
        self.eval::<()>(FINGERPRINTJS_UMD_BUNDLE)?;

        let json = self.eval_async_as_string(
            r#"
            (async () => {
                try {
                    const agent = await FingerprintJS.load({
                        debug: false,
                        monitoring: false,
                    });
                    const result = await agent.get();
                    const componentKeys = Object.keys(result.components).sort();
                    const failedComponents = componentKeys.filter(
                        (key) => "error" in result.components[key],
                    );
                    const failedComponentErrors = failedComponents.map((key) => ({
                        key,
                        name: result.components[key].error?.name ?? "Error",
                        message: String(
                            result.components[key].error?.message ??
                            result.components[key].error,
                        ),
                    }));

                    return {
                        ok: true,
                        version: result.version ?? null,
                        visitor_id: result.visitorId ?? null,
                        confidence_score: result.confidence?.score ?? null,
                        component_count: componentKeys.length,
                        error_component_count: failedComponents.length,
                        component_keys: componentKeys,
                        failed_components: failedComponents,
                        failed_component_errors: failedComponentErrors,
                        top_level_error: null,
                    };
                } catch (error) {
                    return {
                        ok: false,
                        version: null,
                        visitor_id: null,
                        confidence_score: null,
                        component_count: 0,
                        error_component_count: 0,
                        component_keys: [],
                        failed_components: [],
                        failed_component_errors: [],
                        top_level_error: {
                            name: error?.name ?? "Error",
                            message: String(error?.message ?? error),
                        },
                    };
                }
            })()
            "#,
        )?;

        Ok(serde_json::from_str(&json)?)
    }

    pub fn fingerprintjs_version(&self) -> &'static str {
        FINGERPRINTJS_VERSION
    }

    pub fn default_fingerprintjs_harness_snapshot() -> FingerprintJsHarnessReport {
        serde_json::from_str(DEFAULT_FINGERPRINTJS_HARNESS_SNAPSHOT)
            .expect("valid default fingerprintjs harness snapshot")
    }

    pub fn chromium_fingerprintjs_harness_snapshot() -> FingerprintJsHarnessReport {
        serde_json::from_str(BROWSER_CHROMIUM_HARNESS_SNAPSHOT)
            .expect("valid chromium fingerprintjs harness snapshot")
    }
}
