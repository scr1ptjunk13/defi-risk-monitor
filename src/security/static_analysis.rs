use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::error::AppError;

/// Static analysis engine for security vulnerabilities
#[derive(Debug)]
pub struct StaticAnalyzer {
    // Security patterns to detect
    vulnerability_patterns: Vec<VulnerabilityPattern>,

    // Code quality patterns
    quality_patterns: Vec<QualityPattern>,
    // Analyzed files cache
    analysis_cache: HashMap<PathBuf, AnalysisResult>,
}

#[derive(Debug, Clone)]
pub struct VulnerabilityPattern {
    pub name: String,
    pub pattern: Regex,
    pub severity: VulnerabilitySeverity,
    pub category: VulnerabilityCategory,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct QualityPattern {
    pub name: String,
    pub pattern: Regex,
    pub impact: QualityImpact,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilityCategory {
    InputValidation,
    SqlInjection,
    Authentication,
    Authorization,
    CryptographicIssues,
    BusinessLogic,
    DataExposure,
    DenialOfService,
    MemorySafety,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityImpact {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub file_path: PathBuf,
    pub vulnerabilities: Vec<Vulnerability>,
    pub quality_issues: Vec<QualityIssue>,
    pub lines_analyzed: usize,
    pub analysis_time: chrono::DateTime<chrono::Utc>,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub name: String,
    pub severity: VulnerabilitySeverity,
    pub category: VulnerabilityCategory,
    pub line_number: usize,
    pub code_snippet: String,
    pub description: String,
    pub recommendation: String,
    pub cwe_id: Option<u32>, // Common Weakness Enumeration ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub name: String,
    pub impact: QualityImpact,
    pub line_number: usize,
    pub code_snippet: String,
    pub description: String,
}

impl Default for StaticAnalyzer {
    fn default() -> Self {
        let vulnerability_patterns = vec![
            // SQL Injection patterns
            VulnerabilityPattern {
                name: "Potential SQL Injection".to_string(),
                pattern: Regex::new(r#"(?i)(query!|execute)\s*\(\s*[^$].*format!|query.*\+.*user"#).unwrap(),
                severity: VulnerabilitySeverity::High,
                category: VulnerabilityCategory::SqlInjection,
                description: "Direct string concatenation in SQL queries can lead to SQL injection".to_string(),
                recommendation: "Use parameterized queries with sqlx::query! macro and parameter binding".to_string(),
            },
            
            // Hardcoded secrets
            VulnerabilityPattern {
                name: "Hardcoded Secret".to_string(),
                pattern: Regex::new(r#"(?i)(password|secret|key|token)\s*=\s*["'][^"']{8,}["']"#).unwrap(),
                severity: VulnerabilitySeverity::Critical,
                category: VulnerabilityCategory::CryptographicIssues,
                description: "Hardcoded secrets in source code pose security risks".to_string(),
                recommendation: "Use environment variables or secure secret management".to_string(),
            },
            
            // Unsafe unwrap operations
            VulnerabilityPattern {
                name: "Unsafe Unwrap".to_string(),
                pattern: Regex::new(r"\.unwrap\(\)(?!\s*;?\s*//.*test)").unwrap(),
                severity: VulnerabilitySeverity::Medium,
                category: VulnerabilityCategory::MemorySafety,
                description: "Unwrap operations can cause panics in production".to_string(),
                recommendation: "Use proper error handling with Result types and match statements".to_string(),
            },
            
            // Insufficient input validation
            VulnerabilityPattern {
                name: "Missing Input Validation".to_string(),
                pattern: Regex::new(r"(?i)fn.*\(.*user.*:.*String.*\).*\{(?!.*validate)").unwrap(),
                severity: VulnerabilitySeverity::Medium,
                category: VulnerabilityCategory::InputValidation,
                description: "User input functions without validation".to_string(),
                recommendation: "Add comprehensive input validation for all user-provided data".to_string(),
            },
            
            // Weak cryptographic practices
            VulnerabilityPattern {
                name: "Weak Cryptography".to_string(),
                pattern: Regex::new(r"(?i)(md5|sha1|des|rc4)").unwrap(),
                severity: VulnerabilitySeverity::High,
                category: VulnerabilityCategory::CryptographicIssues,
                description: "Use of weak cryptographic algorithms".to_string(),
                recommendation: "Use strong cryptographic algorithms like SHA-256, AES-256".to_string(),
            },
            
            // Information disclosure
            VulnerabilityPattern {
                name: "Information Disclosure".to_string(),
                pattern: Regex::new(r#"(?i)(println!|eprintln!|dbg!)\s*\(.*(?:password|secret|key|token)"#).unwrap(),
                severity: VulnerabilitySeverity::High,
                category: VulnerabilityCategory::DataExposure,
                description: "Sensitive information in debug output".to_string(),
                recommendation: "Remove debug statements containing sensitive data".to_string(),
            },
            
            // Integer overflow risks
            VulnerabilityPattern {
                name: "Integer Overflow Risk".to_string(),
                pattern: Regex::new(r"(?:u8|u16|u32|u64|i8|i16|i32|i64)\s*\+\s*(?:u8|u16|u32|u64|i8|i16|i32|i64)").unwrap(),
                severity: VulnerabilitySeverity::Low,
                category: VulnerabilityCategory::MemorySafety,
                description: "Potential integer overflow in arithmetic operations".to_string(),
                recommendation: "Use checked arithmetic operations or BigDecimal for financial calculations".to_string(),
            },
            
            // Unsafe deserialization
            VulnerabilityPattern {
                name: "Unsafe Deserialization".to_string(),
                pattern: Regex::new(r"(?i)serde_json::from_str.*user").unwrap(),
                severity: VulnerabilitySeverity::Medium,
                category: VulnerabilityCategory::InputValidation,
                description: "Deserializing user input without validation".to_string(),
                recommendation: "Validate and sanitize data before deserialization".to_string(),
            },
        ];

        let quality_patterns = vec![
            QualityPattern {
                name: "TODO/FIXME Comments".to_string(),
                pattern: Regex::new(r"(?i)(todo|fixme|hack|xxx)").unwrap(),
                impact: QualityImpact::Medium,
                description: "Unresolved TODO/FIXME comments indicate incomplete code".to_string(),
            },
            
            QualityPattern {
                name: "Long Function".to_string(),
                pattern: Regex::new(r"fn\s+\w+.*\{").unwrap(),
                impact: QualityImpact::Low,
                description: "Functions should be kept reasonably short for maintainability".to_string(),
            },
            
            QualityPattern {
                name: "Magic Numbers".to_string(),
                pattern: Regex::new(r"\b(?!0|1|2|10|100|1000)\d{3,}\b").unwrap(),
                impact: QualityImpact::Low,
                description: "Magic numbers should be replaced with named constants".to_string(),
            },
        ];

        Self {
            vulnerability_patterns,
            quality_patterns,
            analysis_cache: HashMap::new(),
        }
    }
}

impl StaticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a single file for security vulnerabilities
    pub fn analyze_file(&mut self, file_path: &Path) -> Result<AnalysisResult, AppError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| AppError::InternalError(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut vulnerabilities = Vec::new();
        let mut quality_issues = Vec::new();

        // Analyze for vulnerabilities
        for (line_num, line) in lines.iter().enumerate() {
            for pattern in &self.vulnerability_patterns {
                if pattern.pattern.is_match(line) {
                    vulnerabilities.push(Vulnerability {
                        name: pattern.name.clone(),
                        severity: pattern.severity.clone(),
                        category: pattern.category.clone(),
                        line_number: line_num + 1,
                        code_snippet: line.to_string(),
                        description: pattern.description.clone(),
                        recommendation: pattern.recommendation.clone(),
                        cwe_id: self.get_cwe_id(&pattern.category),
                    });
                }
            }

            // Analyze for quality issues
            for pattern in &self.quality_patterns {
                if pattern.pattern.is_match(line) {
                    // Special handling for long functions
                    if pattern.name == "Long Function" {
                        let function_length = self.count_function_lines(&lines, line_num);
                        if function_length > 50 {
                            quality_issues.push(QualityIssue {
                                name: format!("Long Function ({} lines)", function_length),
                                impact: if function_length > 100 { QualityImpact::High } else { QualityImpact::Medium },
                                line_number: line_num + 1,
                                code_snippet: line.to_string(),
                                description: format!("Function is {} lines long, consider refactoring", function_length),
                            });
                        }
                    } else {
                        quality_issues.push(QualityIssue {
                            name: pattern.name.clone(),
                            impact: pattern.impact.clone(),
                            line_number: line_num + 1,
                            code_snippet: line.to_string(),
                            description: pattern.description.clone(),
                        });
                    }
                }
            }
        }

        let risk_score = self.calculate_risk_score(&vulnerabilities);

        let result = AnalysisResult {
            file_path: file_path.to_path_buf(),
            vulnerabilities,
            quality_issues,
            lines_analyzed: lines.len(),
            analysis_time: chrono::Utc::now(),
            risk_score,
        };

        self.analysis_cache.insert(file_path.to_path_buf(), result.clone());
        Ok(result)
    }

    /// Analyze entire project directory
    pub fn analyze_project(&mut self, project_path: &Path) -> Result<ProjectAnalysisReport, AppError> {
        let mut all_results = Vec::new();
        let mut total_vulnerabilities = 0;
        let mut total_quality_issues = 0;
        let mut critical_files = Vec::new();

        // Find all Rust files
        let rust_files = self.find_rust_files(project_path)?;

        for file_path in rust_files {
            match self.analyze_file(&file_path) {
                Ok(result) => {
                    total_vulnerabilities += result.vulnerabilities.len();
                    total_quality_issues += result.quality_issues.len();
                    
                    if result.risk_score > 7.0 {
                        critical_files.push(file_path.clone());
                    }
                    
                    all_results.push(result);
                }
                Err(e) => {
                    eprintln!("Failed to analyze {}: {}", file_path.display(), e);
                }
            }
        }

        let overall_risk_score = self.calculate_project_risk_score(&all_results);

        let recommendations = self.generate_project_recommendations(&all_results);
        
        Ok(ProjectAnalysisReport {
            project_path: project_path.to_path_buf(),
            file_results: all_results,
            total_vulnerabilities,
            total_quality_issues,
            critical_files,
            overall_risk_score,
            analysis_time: chrono::Utc::now(),
            recommendations,
        })
    }

    /// Generate security report
    pub fn generate_security_report(&self, results: &[AnalysisResult]) -> SecurityReport {
        let mut vulnerability_counts = HashMap::new();
        let mut severity_counts = HashMap::new();
        let mut category_counts = HashMap::new();

        for result in results {
            for vuln in &result.vulnerabilities {
                *vulnerability_counts.entry(vuln.name.clone()).or_insert(0) += 1;
                *severity_counts.entry(format!("{:?}", vuln.severity)).or_insert(0) += 1;
                *category_counts.entry(format!("{:?}", vuln.category)).or_insert(0) += 1;
            }
        }

        let high_risk_files: Vec<_> = results
            .iter()
            .filter(|r| r.risk_score > 7.0)
            .map(|r| r.file_path.clone())
            .collect();

        SecurityReport {
            total_files_analyzed: results.len(),
            total_vulnerabilities: results.iter().map(|r| r.vulnerabilities.len()).sum(),
            vulnerability_counts,
            severity_counts,
            category_counts,
            high_risk_files,
            report_time: chrono::Utc::now(),
        }
    }

    fn find_rust_files(&self, dir: &Path) -> Result<Vec<PathBuf>, AppError> {
        let mut rust_files = Vec::new();
        
        if dir.is_dir() {
            for entry in fs::read_dir(dir)
                .map_err(|e| AppError::InternalError(format!("Failed to read directory: {}", e)))? 
            {
                let entry = entry
                    .map_err(|e| AppError::InternalError(format!("Failed to read entry: {}", e)))?;
                let path = entry.path();
                
                if path.is_dir() && !path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                    rust_files.extend(self.find_rust_files(&path)?);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    rust_files.push(path);
                }
            }
        }
        
        Ok(rust_files)
    }

    fn count_function_lines(&self, lines: &[&str], start_line: usize) -> usize {
        let mut brace_count = 0;
        let mut line_count = 0;
        let mut started = false;

        for line in lines.iter().skip(start_line) {
            line_count += 1;
            
            for ch in line.chars() {
                match ch {
                    '{' => {
                        brace_count += 1;
                        started = true;
                    }
                    '}' => {
                        brace_count -= 1;
                        if started && brace_count == 0 {
                            return line_count;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        line_count
    }

    fn calculate_risk_score(&self, vulnerabilities: &[Vulnerability]) -> f64 {
        let mut score = 0.0;
        
        for vuln in vulnerabilities {
            score += match vuln.severity {
                VulnerabilitySeverity::Critical => 4.0,
                VulnerabilitySeverity::High => 3.0,
                VulnerabilitySeverity::Medium => 2.0,
                VulnerabilitySeverity::Low => 1.0,
            };
        }
        
        // Normalize to 0-10 scale
        (score / vulnerabilities.len().max(1) as f64).min(10.0)
    }

    fn calculate_project_risk_score(&self, results: &[AnalysisResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        
        let total_score: f64 = results.iter().map(|r| r.risk_score).sum();
        total_score / results.len() as f64
    }

    fn get_cwe_id(&self, category: &VulnerabilityCategory) -> Option<u32> {
        match category {
            VulnerabilityCategory::SqlInjection => Some(89),
            VulnerabilityCategory::InputValidation => Some(20),
            VulnerabilityCategory::Authentication => Some(287),
            VulnerabilityCategory::Authorization => Some(285),
            VulnerabilityCategory::CryptographicIssues => Some(327),
            VulnerabilityCategory::DataExposure => Some(200),
            VulnerabilityCategory::DenialOfService => Some(400),
            VulnerabilityCategory::MemorySafety => Some(119),
            VulnerabilityCategory::BusinessLogic => Some(840),
        }
    }

    fn generate_project_recommendations(&self, results: &[AnalysisResult]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut critical_count = 0;
        let mut high_count = 0;

        for result in results {
            for vuln in &result.vulnerabilities {
                match vuln.severity {
                    VulnerabilitySeverity::Critical => critical_count += 1,
                    VulnerabilitySeverity::High => high_count += 1,
                    _ => {}
                }
            }
        }

        if critical_count > 0 {
            recommendations.push(format!("URGENT: Address {} critical vulnerabilities immediately", critical_count));
        }

        if high_count > 0 {
            recommendations.push(format!("High priority: Fix {} high-severity vulnerabilities", high_count));
        }

        recommendations.push("Implement comprehensive input validation across all user-facing endpoints".to_string());
        recommendations.push("Use parameterized queries for all database operations".to_string());
        recommendations.push("Implement proper error handling to avoid panics in production".to_string());
        recommendations.push("Use secure secret management for all sensitive configuration".to_string());

        recommendations
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAnalysisReport {
    pub project_path: PathBuf,
    pub file_results: Vec<AnalysisResult>,
    pub total_vulnerabilities: usize,
    pub total_quality_issues: usize,
    pub critical_files: Vec<PathBuf>,
    pub overall_risk_score: f64,
    pub analysis_time: chrono::DateTime<chrono::Utc>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub total_files_analyzed: usize,
    pub total_vulnerabilities: usize,
    pub vulnerability_counts: HashMap<String, usize>,
    pub severity_counts: HashMap<String, usize>,
    pub category_counts: HashMap<String, usize>,
    pub high_risk_files: Vec<PathBuf>,
    pub report_time: chrono::DateTime<chrono::Utc>,
}

// Re-export key types for use in mod.rs
// (actual re-export is now only in mod.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_sql_injection_detection() {
        let mut analyzer = StaticAnalyzer::new();
        
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "let query = format!(\"SELECT * FROM users WHERE id = {}\", user_id);").unwrap();
        writeln!(temp_file, "sqlx::query(&query).execute(&pool).await;").unwrap();
        
        let result = analyzer.analyze_file(temp_file.path()).unwrap();
        
        // Should detect potential SQL injection
        assert!(!result.vulnerabilities.is_empty());
        assert!(result.vulnerabilities.iter().any(|v| v.name.contains("SQL Injection")));
    }

    #[test]
    fn test_hardcoded_secret_detection() {
        let mut analyzer = StaticAnalyzer::new();
        
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "let password = \"hardcoded_password_123\";").unwrap();
        
        let result = analyzer.analyze_file(temp_file.path()).unwrap();
        
        // Should detect hardcoded secret
        assert!(!result.vulnerabilities.is_empty());
        assert!(result.vulnerabilities.iter().any(|v| v.name.contains("Hardcoded Secret")));
    }

    #[test]
    fn test_unsafe_unwrap_detection() {
        let mut analyzer = StaticAnalyzer::new();
        
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "let value = some_result.unwrap();").unwrap();
        writeln!(temp_file, "let test_value = test_result.unwrap(); // test").unwrap();
        
        let result = analyzer.analyze_file(temp_file.path()).unwrap();
        
        // Should detect unsafe unwrap but not test unwrap
        let unwrap_vulns: Vec<_> = result.vulnerabilities.iter()
            .filter(|v| v.name.contains("Unsafe Unwrap"))
            .collect();
        assert_eq!(unwrap_vulns.len(), 1);
    }

    #[test]
    fn test_risk_score_calculation() {
        let vulnerabilities = vec![
            Vulnerability {
                name: "Critical Issue".to_string(),
                severity: VulnerabilitySeverity::Critical,
                category: VulnerabilityCategory::SqlInjection,
                line_number: 1,
                code_snippet: "test".to_string(),
                description: "test".to_string(),
                recommendation: "test".to_string(),
                cwe_id: Some(89),
            },
            Vulnerability {
                name: "Medium Issue".to_string(),
                severity: VulnerabilitySeverity::Medium,
                category: VulnerabilityCategory::InputValidation,
                line_number: 2,
                code_snippet: "test".to_string(),
                description: "test".to_string(),
                recommendation: "test".to_string(),
                cwe_id: Some(20),
            },
        ];

        let analyzer = StaticAnalyzer::new();
        let risk_score = analyzer.calculate_risk_score(&vulnerabilities);
        
        // Should be average of 4.0 (critical) and 2.0 (medium) = 3.0
        assert_eq!(risk_score, 3.0);
    }
}
