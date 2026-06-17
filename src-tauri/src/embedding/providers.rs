use std::path::Path;

pub const ONNX_RUNTIME_CRATE: &str = "ort";
pub const ONNX_RUNTIME_CRATE_VERSION: &str = "2.0.0-rc.12";
pub const ONNX_RUNTIME_FEATURES: &[&str] = &["load-dynamic", "api-24"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSmokeResult {
    pub available: bool,
    pub detail: String,
}

pub fn smoke_test_session(model_path: &Path) -> SessionSmokeResult {
    if !model_path.exists() {
        return SessionSmokeResult {
            available: false,
            detail: format!("model asset not found: {}", model_path.display()),
        };
    }

    smoke_test_existing_model(model_path)
}

#[cfg(feature = "onnx-runtime")]
fn smoke_test_existing_model(model_path: &Path) -> SessionSmokeResult {
    let detail = format!(
        "{} {} configured with {:?}; session smoke test pending model input metadata for {}.",
        ONNX_RUNTIME_CRATE,
        ONNX_RUNTIME_CRATE_VERSION,
        ONNX_RUNTIME_FEATURES,
        model_path.display()
    );
    SessionSmokeResult {
        available: true,
        detail,
    }
}

#[cfg(not(feature = "onnx-runtime"))]
fn smoke_test_existing_model(model_path: &Path) -> SessionSmokeResult {
    SessionSmokeResult {
        available: false,
        detail: format!(
            "{} {} is configured with {:?}; enable the 'onnx-runtime' Cargo feature to load {}.",
            ONNX_RUNTIME_CRATE,
            ONNX_RUNTIME_CRATE_VERSION,
            ONNX_RUNTIME_FEATURES,
            model_path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test_session_reports_missing_model_asset() {
        let result = smoke_test_session(Path::new("D:\\workspace\\.dataviewer\\models\\missing.onnx"));

        assert!(!result.available);
        assert!(result.detail.contains("model asset not found"));
    }

    #[test]
    fn records_selected_onnx_runtime_binding() {
        assert_eq!(ONNX_RUNTIME_CRATE, "ort");
        assert_eq!(ONNX_RUNTIME_CRATE_VERSION, "2.0.0-rc.12");
        assert_eq!(ONNX_RUNTIME_FEATURES, ["load-dynamic", "api-24"]);
    }
}
