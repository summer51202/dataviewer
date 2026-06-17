#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeBackend {
    Cuda,
    WindowsGpu,
    Cpu,
}

impl RuntimeBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeBackend::Cuda => "cuda",
            RuntimeBackend::WindowsGpu => "windows-gpu",
            RuntimeBackend::Cpu => "cpu",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProbeResult {
    pub selected_backend: RuntimeBackend,
    pub fallback_reason: Option<String>,
}

pub fn probe_runtime(_preference: &str) -> RuntimeProbeResult {
    RuntimeProbeResult {
        selected_backend: RuntimeBackend::Cpu,
        fallback_reason: Some(
            "ONNX Runtime provider probing is not enabled yet; using CPU.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_runtime_returns_stable_cpu_fallback() {
        let result = probe_runtime("cuda");

        assert_eq!(result.selected_backend, RuntimeBackend::Cpu);
        assert!(result.fallback_reason.is_some());
    }
}
