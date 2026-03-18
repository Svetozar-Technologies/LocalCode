use serde::{Deserialize, Serialize};
use std::path::Path;

use super::venv::VenvInfo;
use crate::CoreResult;

/// Pytest test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub nodeid: String,
    pub outcome: TestOutcome,
    pub duration: f64,
    pub file: Option<String>,
    pub line: Option<u64>,
    pub message: Option<String>,
    pub longrepr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestOutcome {
    Passed,
    Failed,
    Error,
    Skipped,
    XFailed,
    XPassed,
}

/// Overall test run summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub skipped: usize,
    pub duration: f64,
    pub results: Vec<TestResult>,
}

/// Run pytest and parse results
pub fn run_tests(
    project_path: &str,
    venv: Option<&VenvInfo>,
    args: &[String],
) -> CoreResult<TestRunSummary> {
    let python = venv
        .map(|v| v.python_path.to_string_lossy().to_string())
        .unwrap_or_else(|| "python3".to_string());

    // Use JSON output for structured results
    let json_report = Path::new(project_path)
        .join(".localcode")
        .join("pytest-report.json");

    // Ensure .localcode directory exists
    if let Some(parent) = json_report.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut cmd_args = vec![
        "-m".to_string(),
        "pytest".to_string(),
        format!("--json-report"),
        format!("--json-report-file={}", json_report.display()),
        "-v".to_string(),
    ];
    cmd_args.extend(args.iter().cloned());

    let output = std::process::Command::new(&python)
        .args(&cmd_args)
        .current_dir(project_path)
        .output()
        .map_err(|e| crate::CoreError::Other(format!("Failed to run pytest: {}", e)))?;

    // Try to parse JSON report first
    if json_report.exists() {
        if let Ok(report) = parse_json_report(&json_report) {
            let _ = std::fs::remove_file(&json_report);
            return Ok(report);
        }
    }

    // Fallback: parse stdout
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(parse_verbose_output(&stdout, &stderr))
}

/// Run a specific test file
pub fn run_test_file(
    project_path: &str,
    venv: Option<&VenvInfo>,
    test_file: &str,
) -> CoreResult<TestRunSummary> {
    run_tests(project_path, venv, &[test_file.to_string()])
}

/// Run a specific test function
pub fn run_test_function(
    project_path: &str,
    venv: Option<&VenvInfo>,
    test_nodeid: &str,
) -> CoreResult<TestRunSummary> {
    run_tests(project_path, venv, &[test_nodeid.to_string()])
}

/// Discover all test files in a project
pub fn discover_tests(project_path: &str) -> CoreResult<Vec<String>> {
    let mut test_files = Vec::new();

    let walker = ignore::WalkBuilder::new(project_path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| crate::CoreError::Other(e.to_string()))?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if (file_name.starts_with("test_") || file_name.ends_with("_test.py"))
            && file_name.ends_with(".py")
        {
            if let Ok(rel) = path.strip_prefix(project_path) {
                test_files.push(rel.to_string_lossy().to_string());
            }
        }
    }

    test_files.sort();
    Ok(test_files)
}

fn parse_json_report(path: &Path) -> CoreResult<TestRunSummary> {
    let content = std::fs::read_to_string(path)?;
    let report: serde_json::Value = serde_json::from_str(&content)?;

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut errors = 0;
    let mut skipped = 0;

    if let Some(tests) = report["tests"].as_array() {
        for test in tests {
            let outcome = match test["outcome"].as_str().unwrap_or("") {
                "passed" => {
                    passed += 1;
                    TestOutcome::Passed
                }
                "failed" => {
                    failed += 1;
                    TestOutcome::Failed
                }
                "error" => {
                    errors += 1;
                    TestOutcome::Error
                }
                "skipped" => {
                    skipped += 1;
                    TestOutcome::Skipped
                }
                "xfailed" => TestOutcome::XFailed,
                "xpassed" => TestOutcome::XPassed,
                _ => TestOutcome::Error,
            };

            let nodeid = test["nodeid"].as_str().unwrap_or("").to_string();
            let name = nodeid.split("::").last().unwrap_or(&nodeid).to_string();

            results.push(TestResult {
                name,
                nodeid,
                outcome,
                duration: test["call"]["duration"].as_f64().unwrap_or(0.0),
                file: test["nodeid"]
                    .as_str()
                    .and_then(|n| n.split("::").next())
                    .map(|s| s.to_string()),
                line: test["lineno"].as_u64(),
                message: test["call"]["longrepr"].as_str().map(|s| {
                    s.lines().next().unwrap_or("").to_string()
                }),
                longrepr: test["call"]["longrepr"].as_str().map(|s| s.to_string()),
            });
        }
    }

    let duration = report["duration"].as_f64().unwrap_or(0.0);

    Ok(TestRunSummary {
        total: results.len(),
        passed,
        failed,
        errors,
        skipped,
        duration,
        results,
    })
}

fn parse_verbose_output(stdout: &str, _stderr: &str) -> TestRunSummary {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut errors = 0;
    let mut skipped = 0;

    for line in stdout.lines() {
        let line = line.trim();

        // Match lines like: test_file.py::test_name PASSED
        if let Some((nodeid, status)) = line.rsplit_once(' ') {
            let nodeid = nodeid.trim();
            let status = status.trim();

            let outcome = match status {
                "PASSED" => {
                    passed += 1;
                    TestOutcome::Passed
                }
                "FAILED" => {
                    failed += 1;
                    TestOutcome::Failed
                }
                "ERROR" => {
                    errors += 1;
                    TestOutcome::Error
                }
                "SKIPPED" => {
                    skipped += 1;
                    TestOutcome::Skipped
                }
                _ => continue,
            };

            let name = nodeid.split("::").last().unwrap_or(nodeid).to_string();
            let file = nodeid.split("::").next().map(|s| s.to_string());

            results.push(TestResult {
                name,
                nodeid: nodeid.to_string(),
                outcome,
                duration: 0.0,
                file,
                line: None,
                message: None,
                longrepr: None,
            });
        }
    }

    TestRunSummary {
        total: results.len(),
        passed,
        failed,
        errors,
        skipped,
        duration: 0.0,
        results,
    }
}
