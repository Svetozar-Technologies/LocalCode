use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manages breakpoints across files
#[derive(Debug, Default)]
pub struct BreakpointManager {
    /// Map of file path -> list of breakpoints
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    next_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: u64,
    pub line: u64,
    pub enabled: bool,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub hit_condition: Option<String>,
    #[serde(default)]
    pub log_message: Option<String>,
    /// Whether the debug adapter verified this breakpoint
    pub verified: bool,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a breakpoint at a specific line in a file
    pub fn add_breakpoint(
        &mut self,
        file: &str,
        line: u64,
        condition: Option<String>,
        hit_condition: Option<String>,
        log_message: Option<String>,
    ) -> &Breakpoint {
        let id = self.next_id;
        self.next_id += 1;

        let bp = Breakpoint {
            id,
            line,
            enabled: true,
            condition,
            hit_condition,
            log_message,
            verified: false,
        };

        self.breakpoints
            .entry(file.to_string())
            .or_default()
            .push(bp);

        self.breakpoints
            .get(file)
            .unwrap()
            .last()
            .unwrap()
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u64) -> bool {
        for breakpoints in self.breakpoints.values_mut() {
            if let Some(pos) = breakpoints.iter().position(|bp| bp.id == id) {
                breakpoints.remove(pos);
                return true;
            }
        }
        false
    }

    /// Remove all breakpoints for a file
    pub fn remove_file_breakpoints(&mut self, file: &str) {
        self.breakpoints.remove(file);
    }

    /// Toggle a breakpoint at a specific line
    pub fn toggle_breakpoint(&mut self, file: &str, line: u64) -> bool {
        if let Some(breakpoints) = self.breakpoints.get_mut(file) {
            if let Some(pos) = breakpoints.iter().position(|bp| bp.line == line) {
                breakpoints.remove(pos);
                return false; // Removed
            }
        }

        self.add_breakpoint(file, line, None, None, None);
        true // Added
    }

    /// Enable/disable a breakpoint
    pub fn set_enabled(&mut self, id: u64, enabled: bool) {
        for breakpoints in self.breakpoints.values_mut() {
            if let Some(bp) = breakpoints.iter_mut().find(|bp| bp.id == id) {
                bp.enabled = enabled;
                return;
            }
        }
    }

    /// Get all breakpoints for a file
    pub fn get_breakpoints(&self, file: &str) -> &[Breakpoint] {
        self.breakpoints.get(file).map_or(&[], |v| v.as_slice())
    }

    /// Get all breakpoints across all files
    pub fn all_breakpoints(&self) -> &HashMap<String, Vec<Breakpoint>> {
        &self.breakpoints
    }

    /// Get enabled breakpoint lines for a file (for DAP setBreakpoints)
    pub fn enabled_lines(&self, file: &str) -> Vec<u64> {
        self.breakpoints
            .get(file)
            .map(|bps| {
                bps.iter()
                    .filter(|bp| bp.enabled)
                    .map(|bp| bp.line)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Mark a breakpoint as verified by the debug adapter
    pub fn set_verified(&mut self, file: &str, line: u64, verified: bool) {
        if let Some(breakpoints) = self.breakpoints.get_mut(file) {
            if let Some(bp) = breakpoints.iter_mut().find(|bp| bp.line == line) {
                bp.verified = verified;
            }
        }
    }

    /// Get files that have breakpoints
    pub fn files_with_breakpoints(&self) -> Vec<&str> {
        self.breakpoints
            .iter()
            .filter(|(_, bps)| !bps.is_empty())
            .map(|(file, _)| file.as_str())
            .collect()
    }

    /// Clear all breakpoints
    pub fn clear_all(&mut self) {
        self.breakpoints.clear();
    }

    /// Save breakpoints to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.breakpoints).unwrap_or_default()
    }

    /// Load breakpoints from JSON
    pub fn from_json(value: &serde_json::Value) -> Self {
        let breakpoints: HashMap<String, Vec<Breakpoint>> =
            serde_json::from_value(value.clone()).unwrap_or_default();

        let max_id = breakpoints
            .values()
            .flatten()
            .map(|bp| bp.id)
            .max()
            .unwrap_or(0);

        Self {
            breakpoints,
            next_id: max_id + 1,
        }
    }
}
