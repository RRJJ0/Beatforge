use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Emitter};

pub const PYTORCH_VERSION: &str = "2.10.0";
pub const FFMPEG_PACKAGE_VERSION: &str = "0.6.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeBackend {
    Cuda130,
    Cuda128,
    Cuda126,
    Cpu,
}

impl ComputeBackend {
    pub fn index_url(self) -> &'static str {
        match self {
            Self::Cuda130 => "https://download.pytorch.org/whl/cu130",
            Self::Cuda128 => "https://download.pytorch.org/whl/cu128",
            Self::Cuda126 => "https://download.pytorch.org/whl/cu126",
            Self::Cpu => "https://download.pytorch.org/whl/cpu",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cuda130 => "CUDA 13.0",
            Self::Cuda128 => "CUDA 12.8",
            Self::Cuda126 => "CUDA 12.6",
            Self::Cpu => "CPU",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDiagnostic {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDiagnostics {
    pub python: ComponentDiagnostic,
    pub pip: ComponentDiagnostic,
    pub ffmpeg: ComponentDiagnostic,
    pub pytorch: ComponentDiagnostic,
    pub gpu_name: Option<String>,
    pub driver_version: Option<String>,
    pub cuda_available: bool,
    pub recommended_backend: ComputeBackend,
    pub compatibility_reason: String,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInstallEvent {
    pub stage: String,
    pub progress: u8,
    pub message: String,
}

pub struct EnvironmentManager {
    busy: AtomicBool,
}

impl EnvironmentManager {
    pub fn new() -> Self {
        Self {
            busy: AtomicBool::new(false),
        }
    }

    pub fn begin(&self) -> Result<(), String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| "AI dependency installation is already running".to_string())
    }

    pub fn finish(&self) {
        self.busy.store(false, Ordering::Release);
    }
}

pub fn recommend_backend(
    gpu_available: bool,
    driver_version: Option<&str>,
) -> (ComputeBackend, String) {
    if !gpu_available {
        return (
            ComputeBackend::Cpu,
            "No NVIDIA GPU was detected. CPU mode works, but generation will be much slower."
                .into(),
        );
    }
    let Some(version) = driver_version.and_then(parse_driver_version) else {
        return (
            ComputeBackend::Cpu,
            "The NVIDIA driver version could not be read, so the safe CPU package is recommended."
                .into(),
        );
    };
    if version >= (580, 0) {
        return (
            ComputeBackend::Cuda130,
            "The installed NVIDIA driver supports the CUDA 13.0 PyTorch package.".into(),
        );
    }
    if version >= (570, 65) {
        return (
            ComputeBackend::Cuda128,
            "The installed NVIDIA driver supports the CUDA 12.8 PyTorch package.".into(),
        );
    }
    if version >= (560, 76) {
        return (
            ComputeBackend::Cuda126,
            "The installed NVIDIA driver supports the CUDA 12.6 PyTorch package.".into(),
        );
    }
    (
        ComputeBackend::Cpu,
        format!(
            "NVIDIA driver {} is below the supported CUDA wheel range. Update the driver to enable GPU generation.",
            driver_version.unwrap_or("unknown")
        ),
    )
}

fn parse_driver_version(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.trim().split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn missing(error: impl Into<String>) -> ComponentDiagnostic {
    ComponentDiagnostic {
        installed: false,
        version: None,
        path: None,
        error: Some(error.into()),
    }
}

fn run_python(python: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(python)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to start {}: {error}", python.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("Python command exited with {}", output.status)
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn inspect_component(python: &Path, script: &str) -> ComponentDiagnostic {
    match run_python(python, &["-I", "-c", script]) {
        Ok(output) => {
            let mut lines = output.lines();
            ComponentDiagnostic {
                installed: true,
                version: lines.next().map(str::to_string),
                path: lines
                    .next()
                    .filter(|line| !line.is_empty())
                    .map(str::to_string),
                error: None,
            }
        }
        Err(error) => missing(error),
    }
}

pub fn diagnose(
    python: Option<&Path>,
    gpu_name: Option<String>,
    driver_version: Option<String>,
) -> EnvironmentDiagnostics {
    let (recommended_backend, compatibility_reason) =
        recommend_backend(gpu_name.is_some(), driver_version.as_deref());
    let Some(python) = python else {
        let runtime_error = "Install the private Python runtime first";
        return EnvironmentDiagnostics {
            python: missing(runtime_error),
            pip: missing(runtime_error),
            ffmpeg: missing(runtime_error),
            pytorch: missing(runtime_error),
            gpu_name,
            driver_version,
            cuda_available: false,
            recommended_backend,
            compatibility_reason,
            ready: false,
        };
    };

    let python_info = inspect_component(
        python,
        "import sys; print(sys.version.split()[0]); print(sys.executable)",
    );
    let pip = inspect_component(
        python,
        "import pip; print(pip.__version__); print(pip.__file__)",
    );
    let ffmpeg = inspect_component(
        python,
        "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_version()); print(imageio_ffmpeg.get_ffmpeg_exe())",
    );
    let pytorch = inspect_component(
        python,
        "import torch; print(torch.__version__); print(torch.version.cuda or 'cpu')",
    );
    let cuda_available = run_python(
        python,
        &[
            "-I",
            "-c",
            "import torch; print('true' if torch.cuda.is_available() else 'false')",
        ],
    )
    .is_ok_and(|value| value == "true");
    let expects_cuda = pytorch.path.as_deref().is_some_and(|value| value != "cpu");
    let ready = python_info.installed
        && pip.installed
        && ffmpeg.installed
        && pytorch.installed
        && (!expects_cuda || cuda_available);

    EnvironmentDiagnostics {
        python: python_info,
        pip,
        ffmpeg,
        pytorch,
        gpu_name,
        driver_version,
        cuda_available,
        recommended_backend,
        compatibility_reason,
        ready,
    }
}

fn emit(app: &AppHandle, stage: &str, progress: u8, message: impl Into<String>) {
    let _ = app.emit(
        "environment-install-event",
        EnvironmentInstallEvent {
            stage: stage.into(),
            progress,
            message: message.into(),
        },
    );
}

fn run_checked(python: &Path, args: &[&str], label: &str) -> Result<(), String> {
    run_python(python, args)
        .map(|_| ())
        .map_err(|error| format!("{label} failed: {error}"))
}

pub fn install_dependencies(
    app: &AppHandle,
    python: &Path,
    backend: ComputeBackend,
) -> Result<(), String> {
    emit(
        app,
        "preparing",
        5,
        "Checking the private Python environment",
    );
    if run_python(python, &["-I", "-m", "pip", "--version"]).is_err() {
        emit(app, "pip", 10, "Bootstrapping pip");
        run_checked(
            python,
            &["-I", "-m", "ensurepip", "--upgrade"],
            "pip bootstrap",
        )?;
    }

    emit(app, "ffmpeg", 20, "Installing the managed FFmpeg binary");
    let ffmpeg_requirement = format!("imageio-ffmpeg=={FFMPEG_PACKAGE_VERSION}");
    run_checked(
        python,
        &[
            "-I",
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--only-binary=:all:",
            &ffmpeg_requirement,
        ],
        "FFmpeg installation",
    )?;

    emit(
        app,
        "pytorch",
        45,
        format!(
            "Installing PyTorch {PYTORCH_VERSION} for {}",
            backend.label()
        ),
    );
    let torch_requirement = format!("torch=={PYTORCH_VERSION}");
    let audio_requirement = format!("torchaudio=={PYTORCH_VERSION}");
    run_checked(
        python,
        &[
            "-I",
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--only-binary=:all:",
            "--index-url",
            backend.index_url(),
            &torch_requirement,
            &audio_requirement,
        ],
        "PyTorch installation",
    )?;

    emit(app, "verifying", 90, "Verifying PyTorch and FFmpeg");
    run_checked(
        python,
        &[
            "-I",
            "-c",
            "import imageio_ffmpeg, torch, torchaudio; assert imageio_ffmpeg.get_ffmpeg_exe(); print(torch.__version__)",
        ],
        "Dependency verification",
    )?;
    emit(app, "completed", 100, "AI dependencies are ready");
    Ok(())
}

pub fn python_executable(install_path: Option<&str>) -> Option<PathBuf> {
    install_path
        .map(PathBuf::from)
        .map(|path| path.join("python.exe"))
        .filter(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_tested_backend_for_driver() {
        assert_eq!(
            recommend_backend(true, Some("581.15")).0,
            ComputeBackend::Cuda130
        );
        assert_eq!(
            recommend_backend(true, Some("572.61")).0,
            ComputeBackend::Cuda128
        );
        assert_eq!(
            recommend_backend(true, Some("566.36")).0,
            ComputeBackend::Cuda126
        );
    }

    #[test]
    fn falls_back_to_cpu_for_old_or_missing_driver() {
        assert_eq!(
            recommend_backend(true, Some("552.22")).0,
            ComputeBackend::Cpu
        );
        assert_eq!(recommend_backend(true, None).0, ComputeBackend::Cpu);
        assert_eq!(
            recommend_backend(false, Some("581.15")).0,
            ComputeBackend::Cpu
        );
    }

    #[test]
    fn backend_indexes_are_allowlisted() {
        for backend in [
            ComputeBackend::Cuda130,
            ComputeBackend::Cuda128,
            ComputeBackend::Cuda126,
            ComputeBackend::Cpu,
        ] {
            assert!(
                backend
                    .index_url()
                    .starts_with("https://download.pytorch.org/whl/")
            );
        }
    }
}
