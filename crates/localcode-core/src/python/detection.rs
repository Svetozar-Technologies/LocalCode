use std::path::Path;

use serde::{Deserialize, Serialize};

/// Detected Python project type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PythonFramework {
    Django,
    Flask,
    FastAPI,
    Starlette,
    Tornado,
    Pyramid,
    Plain,
    Unknown,
}

/// Python project detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonProject {
    pub is_python: bool,
    pub framework: PythonFramework,
    pub has_pyproject: bool,
    pub has_setup_py: bool,
    pub has_setup_cfg: bool,
    pub has_requirements: bool,
    pub has_pipfile: bool,
    pub has_poetry: bool,
    pub test_framework: Option<String>,
    pub python_version: Option<String>,
    pub entry_point: Option<String>,
}

/// Detect Python project characteristics
pub fn detect_python_project(project_path: &str) -> PythonProject {
    let project = Path::new(project_path);

    let has_pyproject = project.join("pyproject.toml").exists();
    let has_setup_py = project.join("setup.py").exists();
    let has_setup_cfg = project.join("setup.cfg").exists();
    let has_requirements = project.join("requirements.txt").exists();
    let has_pipfile = project.join("Pipfile").exists();

    let is_python = has_pyproject
        || has_setup_py
        || has_setup_cfg
        || has_requirements
        || has_pipfile
        || has_any_py_files(project_path);

    if !is_python {
        return PythonProject {
            is_python: false,
            framework: PythonFramework::Unknown,
            has_pyproject,
            has_setup_py,
            has_setup_cfg,
            has_requirements,
            has_pipfile,
            has_poetry: false,
            test_framework: None,
            python_version: None,
            entry_point: None,
        };
    }

    let has_poetry = has_pyproject && {
        std::fs::read_to_string(project.join("pyproject.toml"))
            .map(|c| c.contains("[tool.poetry]"))
            .unwrap_or(false)
    };

    let framework = detect_framework(project_path);
    let test_framework = detect_test_framework(project_path);
    let python_version = detect_python_version(project_path);
    let entry_point = detect_entry_point(project_path, &framework);

    PythonProject {
        is_python,
        framework,
        has_pyproject,
        has_setup_py,
        has_setup_cfg,
        has_requirements,
        has_pipfile,
        has_poetry,
        test_framework,
        python_version,
        entry_point,
    }
}

fn has_any_py_files(project_path: &str) -> bool {
    let walker = ignore::WalkBuilder::new(project_path)
        .hidden(true)
        .git_ignore(true)
        .max_depth(Some(2))
        .build();

    for entry in walker.flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) == Some("py") {
            return true;
        }
    }
    false
}

fn detect_framework(project_path: &str) -> PythonFramework {
    let project = Path::new(project_path);

    // Django: manage.py or django in requirements
    if project.join("manage.py").exists() {
        return PythonFramework::Django;
    }

    // Check requirements/pyproject for framework imports
    let deps = read_dependencies(project_path);

    if deps.contains("django") {
        return PythonFramework::Django;
    }
    if deps.contains("fastapi") {
        return PythonFramework::FastAPI;
    }
    if deps.contains("flask") {
        return PythonFramework::Flask;
    }
    if deps.contains("starlette") {
        return PythonFramework::Starlette;
    }
    if deps.contains("tornado") {
        return PythonFramework::Tornado;
    }
    if deps.contains("pyramid") {
        return PythonFramework::Pyramid;
    }

    // Check for common patterns in source files
    let walker = ignore::WalkBuilder::new(project_path)
        .hidden(true)
        .git_ignore(true)
        .max_depth(Some(3))
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("py") {
            if let Ok(content) = std::fs::read_to_string(path) {
                if content.contains("from django") || content.contains("import django") {
                    return PythonFramework::Django;
                }
                if content.contains("from fastapi") || content.contains("import fastapi") {
                    return PythonFramework::FastAPI;
                }
                if content.contains("from flask") || content.contains("import flask") {
                    return PythonFramework::Flask;
                }
            }
        }
    }

    PythonFramework::Plain
}

fn detect_test_framework(project_path: &str) -> Option<String> {
    let project = Path::new(project_path);

    // Check for pytest.ini, conftest.py, setup.cfg [tool:pytest]
    if project.join("pytest.ini").exists() || project.join("conftest.py").exists() {
        return Some("pytest".to_string());
    }

    if project.join("setup.cfg").exists() {
        if let Ok(content) = std::fs::read_to_string(project.join("setup.cfg")) {
            if content.contains("[tool:pytest]") {
                return Some("pytest".to_string());
            }
        }
    }

    if project.join("pyproject.toml").exists() {
        if let Ok(content) = std::fs::read_to_string(project.join("pyproject.toml")) {
            if content.contains("[tool.pytest") {
                return Some("pytest".to_string());
            }
        }
    }

    // Check requirements for pytest
    let deps = read_dependencies(project_path);
    if deps.contains("pytest") {
        return Some("pytest".to_string());
    }

    // Check for unittest patterns
    let walker = ignore::WalkBuilder::new(project_path)
        .hidden(true)
        .git_ignore(true)
        .max_depth(Some(3))
        .build();

    for entry in walker.flatten() {
        let name = entry
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name.starts_with("test_") && name.ends_with(".py") {
            return Some("pytest".to_string()); // Default assumption
        }
    }

    None
}

fn detect_python_version(project_path: &str) -> Option<String> {
    let project = Path::new(project_path);

    // Check .python-version
    let pv = project.join(".python-version");
    if pv.exists() {
        if let Ok(version) = std::fs::read_to_string(pv) {
            return Some(version.trim().to_string());
        }
    }

    // Check pyproject.toml requires-python
    if let Ok(content) = std::fs::read_to_string(project.join("pyproject.toml")) {
        for line in content.lines() {
            if line.trim().starts_with("requires-python") {
                if let Some(version) = line.split('=').next_back() {
                    return Some(version.trim().trim_matches('"').to_string());
                }
            }
        }
    }

    None
}

fn detect_entry_point(project_path: &str, framework: &PythonFramework) -> Option<String> {
    let project = Path::new(project_path);

    match framework {
        PythonFramework::Django => {
            if project.join("manage.py").exists() {
                Some("manage.py".to_string())
            } else {
                None
            }
        }
        PythonFramework::Flask | PythonFramework::FastAPI => {
            // Look for app.py, main.py, wsgi.py
            for name in &["app.py", "main.py", "wsgi.py", "asgi.py"] {
                if project.join(name).exists() {
                    return Some(name.to_string());
                }
            }
            // Look for src/ variants
            for name in &["src/app.py", "src/main.py"] {
                if project.join(name).exists() {
                    return Some(name.to_string());
                }
            }
            None
        }
        _ => {
            if project.join("main.py").exists() {
                Some("main.py".to_string())
            } else if project.join("app.py").exists() {
                Some("app.py".to_string())
            } else {
                None
            }
        }
    }
}

fn read_dependencies(project_path: &str) -> String {
    let project = Path::new(project_path);
    let mut deps = String::new();

    // Read requirements.txt
    if let Ok(content) = std::fs::read_to_string(project.join("requirements.txt")) {
        deps.push_str(&content.to_lowercase());
    }

    // Read pyproject.toml
    if let Ok(content) = std::fs::read_to_string(project.join("pyproject.toml")) {
        deps.push_str(&content.to_lowercase());
    }

    // Read Pipfile
    if let Ok(content) = std::fs::read_to_string(project.join("Pipfile")) {
        deps.push_str(&content.to_lowercase());
    }

    deps
}
