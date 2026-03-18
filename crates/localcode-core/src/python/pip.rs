use std::path::Path;

use super::venv::VenvInfo;
use crate::CoreResult;

/// Installed package info
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub location: Option<String>,
}

/// List installed packages in a virtual environment
pub fn list_packages(venv: &VenvInfo) -> CoreResult<Vec<PackageInfo>> {
    let output = std::process::Command::new(&venv.pip_path)
        .args(["list", "--format=json"])
        .output()
        .map_err(|e| crate::CoreError::Other(format!("Failed to run pip list: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::CoreError::Other(format!(
            "pip list failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).unwrap_or_default();

    Ok(packages
        .iter()
        .map(|p| PackageInfo {
            name: p["name"].as_str().unwrap_or("").to_string(),
            version: p["version"].as_str().unwrap_or("").to_string(),
            location: p["location"].as_str().map(|s| s.to_string()),
        })
        .collect())
}

/// Install a package
pub fn install_package(venv: &VenvInfo, package: &str) -> CoreResult<String> {
    let output = std::process::Command::new(&venv.pip_path)
        .args(["install", package])
        .output()
        .map_err(|e| crate::CoreError::Other(format!("Failed to run pip install: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(crate::CoreError::Other(format!(
            "pip install failed: {}",
            stderr
        )));
    }

    Ok(format!("{}{}", stdout, stderr))
}

/// Install from requirements file
pub fn install_requirements(venv: &VenvInfo, requirements_path: &str) -> CoreResult<String> {
    let output = std::process::Command::new(&venv.pip_path)
        .args(["install", "-r", requirements_path])
        .output()
        .map_err(|e| {
            crate::CoreError::Other(format!("Failed to install requirements: {}", e))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(crate::CoreError::Other(format!(
            "Requirements install failed: {}",
            stderr
        )));
    }

    Ok(format!("{}{}", stdout, stderr))
}

/// Uninstall a package
pub fn uninstall_package(venv: &VenvInfo, package: &str) -> CoreResult<String> {
    let output = std::process::Command::new(&venv.pip_path)
        .args(["uninstall", "-y", package])
        .output()
        .map_err(|e| crate::CoreError::Other(format!("Failed to run pip uninstall: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::CoreError::Other(format!(
            "pip uninstall failed: {}",
            stderr
        )));
    }

    Ok(stdout)
}

/// Check for outdated packages
pub fn check_outdated(venv: &VenvInfo) -> CoreResult<Vec<PackageInfo>> {
    let output = std::process::Command::new(&venv.pip_path)
        .args(["list", "--outdated", "--format=json"])
        .output()
        .map_err(|e| {
            crate::CoreError::Other(format!("Failed to check outdated packages: {}", e))
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).unwrap_or_default();

    Ok(packages
        .iter()
        .map(|p| PackageInfo {
            name: p["name"].as_str().unwrap_or("").to_string(),
            version: p["latest_version"].as_str().unwrap_or("").to_string(),
            location: None,
        })
        .collect())
}

/// Freeze packages to requirements format
pub fn freeze(venv: &VenvInfo) -> CoreResult<String> {
    let output = std::process::Command::new(&venv.pip_path)
        .args(["freeze"])
        .output()
        .map_err(|e| crate::CoreError::Other(format!("Failed to run pip freeze: {}", e)))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Detect requirements files in a project
pub fn detect_requirements(project_path: &str) -> Vec<String> {
    let project = Path::new(project_path);
    let candidates = [
        "requirements.txt",
        "requirements-dev.txt",
        "requirements/base.txt",
        "requirements/dev.txt",
        "requirements/production.txt",
        "requirements/test.txt",
    ];

    candidates
        .iter()
        .filter(|name| project.join(name).exists())
        .map(|s| s.to_string())
        .collect()
}
