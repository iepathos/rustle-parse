//! Coverage reporting utilities for rustle-parse

use anyhow::Result;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoverageError {
    #[error("Failed to generate coverage report: {0}")]
    GenerationFailed(String),

    #[error("Failed to export coverage data: {0}")]
    ExportFailed(String),

    #[error("Coverage data parsing error: {0}")]
    ParseError(String),
}

/// Coverage report data structure
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub function_coverage: f64,
    pub uncovered_lines: Vec<(String, u32)>,
}

impl CoverageReport {
    /// Generate a coverage report from tarpaulin output
    pub fn generate() -> Result<Self, CoverageError> {
        // For now, return a mock report since this would require running tarpaulin
        // In a real implementation, this would parse the JSON output from tarpaulin
        Ok(CoverageReport {
            line_coverage: 51.90,
            branch_coverage: 45.0,
            function_coverage: 60.0,
            uncovered_lines: vec![
                ("src/parser/cache.rs".to_string(), 5),
                ("src/parser/dependency.rs".to_string(), 31),
                ("src/parser/vault.rs".to_string(), 4),
            ],
        })
    }

    /// Check if the coverage meets the given threshold
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.line_coverage >= threshold
    }

    /// Export coverage report to HTML format
    pub fn export_html(&self, path: &Path) -> Result<(), CoverageError> {
        std::fs::create_dir_all(path.parent().unwrap_or(path))
            .map_err(|e| CoverageError::ExportFailed(e.to_string()))?;

        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Coverage Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .metric {{ margin: 20px 0; }}
        .coverage-good {{ color: green; }}
        .coverage-poor {{ color: red; }}
    </style>
</head>
<body>
    <h1>Coverage Report</h1>
    <div class="metric">
        <strong>Line Coverage:</strong> 
        <span class="{}">{:.2}%</span>
    </div>
    <div class="metric">
        <strong>Branch Coverage:</strong> 
        <span class="{}">{:.2}%</span>
    </div>
    <div class="metric">
        <strong>Function Coverage:</strong> 
        <span class="{}">{:.2}%</span>
    </div>
    <h2>Uncovered Files</h2>
    <ul>
    {}
    </ul>
</body>
</html>"#,
            if self.line_coverage >= 85.0 {
                "coverage-good"
            } else {
                "coverage-poor"
            },
            self.line_coverage,
            if self.branch_coverage >= 80.0 {
                "coverage-good"
            } else {
                "coverage-poor"
            },
            self.branch_coverage,
            if self.function_coverage >= 90.0 {
                "coverage-good"
            } else {
                "coverage-poor"
            },
            self.function_coverage,
            self.uncovered_lines
                .iter()
                .map(|(file, lines)| format!("<li>{file}: {lines} uncovered lines</li>"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        std::fs::write(path, html_content)
            .map_err(|e| CoverageError::ExportFailed(e.to_string()))?;

        Ok(())
    }
}
