use std::path::{Path, PathBuf};

use crate::CoreResult;

/// Virtual environment information
#[derive(Debug, Clone)]
pub struct VenvInfo {
    pub path: PathBuf,
    pub python_path: PathBuf,
    pub pip_path: PathBuf,
    pub name: String,
    pub python_version: Option<String>,
    pub is_active: bool,
}

/// Detect virtual environments in a project
pub fn detect_venv(project_path: &str) -> Option<VenvInfo> {
    let project = Path::new(project_path);

    // Common venv directory names
    let venv_names = ["venv", ".venv", "env", ".env", "virtualenv"];

    for name in &venv_names {
        let venv_path = project.join(name);
        if is_venv(&venv_path) {
            return Some(build_venv_info(&venv_path, name));
        }
    }

    // Check for conda env
    let conda_env = project.join("environment.yml");
    if conda_env.exists() {
        // Check if conda prefix is set
        if let Ok(prefix) = std::env::var("CONDA_PREFIX") {
            let conda_path = PathBuf::from(prefix);
            if conda_path.exists() {
                return Some(build_venv_info(&conda_path, "conda"));
            }
        }
    }

    // Check for poetry
    let pyproject = project.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            if content.contains("[tool.poetry]") {
                // Poetry stores venvs in a cache directory
                if let Some(poetry_venv) = find_poetry_venv(project_path) {
                    return Some(poetry_venv);
                }
            }
        }
    }

    None
}

fn is_venv(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    // Check for pyvenv.cfg (standard indicator)
    if path.join("pyvenv.cfg").exists() {
        return true;
    }

    // Check for bin/python or Scripts/python.exe
    let python_bin = if cfg!(windows) {
        path.join("Scripts").join("python.exe")
    } else {
        path.join("bin").join("python")
    };

    python_bin.exists()
}

fn build_venv_info(venv_path: &Path, name: &str) -> VenvInfo {
    let (bin_dir, python_name, pip_name) = if cfg!(windows) {
        ("Scripts", "python.exe", "pip.exe")
    } else {
        ("bin", "python", "pip")
    };

    let python_path = venv_path.join(bin_dir).join(python_name);
    let pip_path = venv_path.join(bin_dir).join(pip_name);

    let python_version = get_python_version(&python_path);

    // Check if this venv is currently active
    let is_active = std::env::var("VIRTUAL_ENV")
        .ok()
        .map(|v| PathBuf::from(v) == venv_path)
        .unwrap_or(false);

    VenvInfo {
        path: venv_path.to_path_buf(),
        python_path,
        pip_path,
        name: name.to_string(),
        python_version,
        is_active,
    }
}

fn get_python_version(python_path: &Path) -> Option<String> {
    if !python_path.exists() {
        return None;
    }

    std::process::Command::new(python_path)
        .args(["--version"])
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        })
}

fn find_poetry_venv(project_path: &str) -> Option<VenvInfo> {
    let output = std::process::Command::new("poetry")
        .args(["env", "info", "--path"])
        .current_dir(project_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let venv_path = String::from_utf8(output.stdout).ok()?;
    let venv_path = PathBuf::from(venv_path.trim());

    if venv_path.exists() {
        Some(build_venv_info(&venv_path, "poetry"))
    } else {
        None
    }
}

/// Create a new virtual environment
pub fn create_venv(project_path: &str, name: &str) -> CoreResult<VenvInfo> {
    let venv_path = Path::new(project_path).join(name);

    let output = std::process::Command::new("python3")
        .args(["-m", "venv", &venv_path.to_string_lossy()])
        .current_dir(project_path)
        .output()
        .map_err(|e| crate::CoreError::Other(format!("Failed to create venv: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::CoreError::Other(format!(
            "Failed to create venv: {}",
            stderr
        )));
    }

    Ok(build_venv_info(&venv_path, name))
}

/// Get the activation command for a venv
pub fn activation_command(venv: &VenvInfo) -> String {
    if cfg!(windows) {
        format!("{}\\Scripts\\activate", venv.path.display())
    } else {
        format!("source {}/bin/activate", venv.path.display())
    }
}
