# L3-CDD-04: QualityGates Component Design Document

**Document ID:** L3-CDD-QG-001
**Component Name:** QualityGateValidator
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L2-ICD-03-ComponentInterfaces.md
**Traceability:** L1-SAD REQ-SYS-003 (Quality Gates), MO-002 (Quality Standardization)

---

## 1. Component Overview

### 1.1 Purpose
Validates workflow phase outputs against business rules to prevent generic/placeholder text, ensure data completeness, and enforce quality standards before delivery to sales team.

### 1.2 Responsibilities
- Execute quality checks on phase outputs (markdown, structured data)
- Block low-quality outputs from proceeding to next phase
- Provide actionable failure reasons for regeneration prompts
- Track quality metrics per session (pass/fail rates)
- Support configurable severity levels (ERROR blocks, WARNING logs)

### 1.3 Integration Points
| Component | Interface | Direction |
|-----------|-----------|-----------|
| AgentOrchestrator | `validate(phase_id, output)` | ← Receives validation requests |
| LLMClient | Regeneration prompts | → Triggers retry with quality feedback |
| StateManager | Quality metrics | → Stores pass/fail statistics |

---

## 2. File Structure

```
src-tauri/src/
├── quality/
│   ├── mod.rs                    # Public API + QualityGateValidator
│   ├── gates/
│   │   ├── mod.rs                # Gate trait + registry
│   │   ├── no_generic_text.rs    # Detects placeholder text
│   │   ├── coverage_quantification.rs # Requires metrics
│   │   ├── roi_present.rs        # Validates ROI calculations
│   │   ├── case_study_present.rs # Ensures specific examples
│   │   ├── contact_validation.rs # Validates contact data
│   │   └── markdown_format.rs    # Validates output structure
│   ├── rules.rs                  # Rule definitions (regex, keywords)
│   └── types.rs                  # ValidationResult, GateSeverity
```

### 2.1 Module Dependencies
```rust
// quality/mod.rs
mod gates;
mod rules;
mod types;

pub use types::{ValidationResult, ValidationFailure, GateSeverity};
pub use self::QualityGateValidator;
```

---

## 3. Data Structures

### 3.1 Core Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation status
    pub passed: bool,

    /// Gate failures (empty if passed = true)
    pub failures: Vec<ValidationFailure>,

    /// Warnings (non-blocking issues)
    pub warnings: Vec<ValidationWarning>,

    /// Quality score (0-100)
    pub quality_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFailure {
    /// Gate name (e.g., "no_generic_text")
    pub gate_name: String,

    /// Severity (ERROR blocks, WARNING logs)
    pub severity: GateSeverity,

    /// Human-readable reason
    pub reason: String,

    /// Specific examples from output
    pub examples: Vec<String>,

    /// Suggested fix for LLM regeneration
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub gate_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GateSeverity {
    /// Blocks phase completion (fails validation)
    ERROR,

    /// Logs issue but allows phase to continue
    WARNING,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub total_validations: usize,
    pub total_passes: usize,
    pub total_failures: usize,
    pub pass_rate: f64,
    pub failures_by_gate: std::collections::HashMap<String, usize>,
}
```

---

## 4. QualityGateValidator Implementation

### 4.1 Main Struct

```rust
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

pub struct QualityGateValidator {
    /// Registered quality gates
    gates: HashMap<String, Arc<dyn QualityGate>>,

    /// Validation history
    validation_log: Vec<ValidationLogEntry>,

    /// Quality metrics
    metrics: QualityMetrics,
}

#[derive(Debug, Clone)]
struct ValidationLogEntry {
    pub phase_id: String,
    pub timestamp: u64,
    pub passed: bool,
    pub quality_score: u8,
    pub failed_gates: Vec<String>,
}
```

### 4.2 Constructor

```rust
impl QualityGateValidator {
    pub fn new() -> Self {
        let mut gates: HashMap<String, Arc<dyn QualityGate>> = HashMap::new();

        // Register all quality gates
        gates.insert(
            "no_generic_text".to_string(),
            Arc::new(NoGenericTextGate::new())
        );
        gates.insert(
            "coverage_quantification".to_string(),
            Arc::new(CoverageQuantificationGate::new())
        );
        gates.insert(
            "roi_present".to_string(),
            Arc::new(RoiPresentGate::new())
        );
        gates.insert(
            "case_study_present".to_string(),
            Arc::new(CaseStudyPresentGate::new())
        );
        gates.insert(
            "contact_validation".to_string(),
            Arc::new(ContactValidationGate::new())
        );
        gates.insert(
            "markdown_format".to_string(),
            Arc::new(MarkdownFormatGate::new())
        );

        Self {
            gates,
            validation_log: Vec::new(),
            metrics: QualityMetrics {
                total_validations: 0,
                total_passes: 0,
                total_failures: 0,
                pass_rate: 0.0,
                failures_by_gate: HashMap::new(),
            },
        }
    }
}
```

### 4.3 Main Validation Method

```rust
impl QualityGateValidator {
    /// Validate phase output against all applicable gates
    pub async fn validate(
        &mut self,
        phase_id: &str,
        output: &str,
    ) -> Result<ValidationResult> {
        let started_at = Self::current_timestamp_ms();

        // Determine which gates apply to this phase
        let applicable_gates = self.get_applicable_gates(phase_id);

        let mut failures = Vec::new();
        let mut warnings = Vec::new();

        // Execute each gate
        for gate_name in applicable_gates {
            let gate = self.gates
                .get(gate_name)
                .expect("Gate not registered")
                .clone();

            match gate.check(output).await {
                Ok(result) if !result.passed => {
                    if result.severity == GateSeverity::ERROR {
                        failures.push(result);
                    } else {
                        warnings.push(ValidationWarning {
                            gate_name: gate_name.to_string(),
                            message: result.reason.clone(),
                        });
                    }
                }
                Err(e) => {
                    // Gate execution error - treat as validation failure
                    failures.push(ValidationFailure {
                        gate_name: gate_name.to_string(),
                        severity: GateSeverity::ERROR,
                        reason: format!("Gate execution error: {}", e),
                        examples: vec![],
                        suggested_fix: "Check gate implementation".to_string(),
                    });
                }
                _ => {} // Passed
            }
        }

        let passed = failures.is_empty();
        let quality_score = self.calculate_quality_score(&failures, &warnings);

        // Log validation
        self.validation_log.push(ValidationLogEntry {
            phase_id: phase_id.to_string(),
            timestamp: started_at,
            passed,
            quality_score,
            failed_gates: failures.iter().map(|f| f.gate_name.clone()).collect(),
        });

        // Update metrics
        self.metrics.total_validations += 1;
        if passed {
            self.metrics.total_passes += 1;
        } else {
            self.metrics.total_failures += 1;
            for failure in &failures {
                *self.metrics.failures_by_gate
                    .entry(failure.gate_name.clone())
                    .or_insert(0) += 1;
            }
        }
        self.metrics.pass_rate = self.metrics.total_passes as f64
            / self.metrics.total_validations as f64;

        Ok(ValidationResult {
            passed,
            failures,
            warnings,
            quality_score,
        })
    }

    /// Get gates applicable to specific phase
    fn get_applicable_gates(&self, phase_id: &str) -> Vec<&str> {
        match phase_id {
            "phase_1" => vec![
                "no_generic_text",
                "contact_validation",
            ],
            "phase_2" => vec![
                "no_generic_text",
                "coverage_quantification",
            ],
            "phase_3" => vec![
                "no_generic_text",
            ],
            "phase_4" => vec![
                "no_generic_text",
                "case_study_present",
            ],
            "phase_5" => vec![
                "no_generic_text",
                "roi_present",
                "markdown_format",
            ],
            _ => vec!["no_generic_text"], // Default
        }
    }

    /// Calculate quality score (0-100)
    fn calculate_quality_score(
        &self,
        failures: &[ValidationFailure],
        warnings: &[ValidationWarning],
    ) -> u8 {
        let error_penalty = failures.len() * 25; // Each error -25 points
        let warning_penalty = warnings.len() * 5; // Each warning -5 points
        let total_penalty = error_penalty + warning_penalty;

        100_u8.saturating_sub(total_penalty as u8)
    }

    /// Get quality metrics
    pub fn metrics(&self) -> &QualityMetrics {
        &self.metrics
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}
```

---

## 5. QualityGate Trait

### 5.1 Trait Definition

```rust
// quality/gates/mod.rs
use async_trait::async_trait;
use anyhow::Result;
use super::types::{ValidationFailure, GateSeverity};

#[async_trait]
pub trait QualityGate: Send + Sync {
    /// Gate name (e.g., "no_generic_text")
    fn name(&self) -> &str;

    /// Gate description
    fn description(&self) -> &str;

    /// Check output against gate rules
    async fn check(&self, output: &str) -> Result<ValidationFailure>;
}
```

---

## 6. Quality Gate Implementations

### 6.1 NoGenericTextGate

```rust
// quality/gates/no_generic_text.rs
use regex::Regex;
use super::{QualityGate, ValidationFailure, GateSeverity};

pub struct NoGenericTextGate {
    /// Patterns indicating generic/placeholder text
    generic_patterns: Vec<Regex>,
}

impl NoGenericTextGate {
    pub fn new() -> Self {
        let patterns = vec![
            // Generic company references
            r"(?i)\b(the company|this company|their company|your company)\b",
            r"(?i)\b(the organization|this organization)\b",
            r"(?i)\b(the business|this business)\b",

            // Placeholder text
            r"(?i)\[.*?\]", // [Company Name], [Industry]
            r"(?i)\{.*?\}", // {company}, {industry}
            r"(?i)<.*?>",   // <company>, <industry>
            r"(?i)XXX|YYY|ZZZ",
            r"(?i)TODO|FIXME|PLACEHOLDER",

            // Vague time references
            r"(?i)\b(recently|lately|in recent times)\b",
            r"(?i)\b(in the past|previously)\b",

            // Generic statistics
            r"(?i)\b(many|several|numerous|various)\b companies",
            r"(?i)\b(significant|substantial|considerable)\b (growth|increase|improvement)",

            // Weasel words without quantification
            r"(?i)\b(may|might|could|possibly|potentially)\b (help|improve|increase)",
        ];

        Self {
            generic_patterns: patterns
                .into_iter()
                .map(|p| Regex::new(p).unwrap())
                .collect(),
        }
    }
}

#[async_trait]
impl QualityGate for NoGenericTextGate {
    fn name(&self) -> &str {
        "no_generic_text"
    }

    fn description(&self) -> &str {
        "Detects generic/placeholder text that should be specific to target company"
    }

    async fn check(&self, output: &str) -> Result<ValidationFailure> {
        let mut examples = Vec::new();

        for pattern in &self.generic_patterns {
            for mat in pattern.find_iter(output) {
                examples.push(mat.as_str().to_string());
            }
        }

        if examples.is_empty() {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "Passed".to_string(),
                examples: vec![],
                suggested_fix: String::new(),
                passed: true, // Add passed field to ValidationFailure
            })
        } else {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: format!(
                    "Found {} instances of generic/placeholder text",
                    examples.len()
                ),
                examples: examples.into_iter().take(5).collect(), // Limit to 5 examples
                suggested_fix: "Replace generic references with specific company name, dates, metrics, and concrete examples".to_string(),
                passed: false,
            })
        }
    }
}
```

### 6.2 CoverageQuantificationGate

```rust
// quality/gates/coverage_quantification.rs
use regex::Regex;

pub struct CoverageQuantificationGate {
    /// Patterns requiring quantification
    number_pattern: Regex,
    metric_pattern: Regex,
}

impl CoverageQuantificationGate {
    pub fn new() -> Self {
        Self {
            number_pattern: Regex::new(r"\b\d+\b").unwrap(),
            metric_pattern: Regex::new(
                r"(?i)\b(articles?|mentions?|pieces?|stories|posts?|views?|impressions?)\b"
            ).unwrap(),
        }
    }
}

#[async_trait]
impl QualityGate for CoverageQuantificationGate {
    fn name(&self) -> &str {
        "coverage_quantification"
    }

    fn description(&self) -> &str {
        "Ensures media coverage is quantified (e.g., '47 articles in Q4 2024')"
    }

    async fn check(&self, output: &str) -> Result<ValidationFailure> {
        // Check if output contains media coverage metrics
        let has_metrics = self.metric_pattern.is_match(output);
        let has_numbers = self.number_pattern.is_match(output);

        if has_metrics && has_numbers {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "Passed".to_string(),
                examples: vec![],
                suggested_fix: String::new(),
                passed: true,
            })
        } else {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "Coverage volume not quantified (missing specific counts)".to_string(),
                examples: vec![
                    "Expected: '47 articles in Q4 2024'".to_string(),
                    "Not: 'significant media coverage'".to_string(),
                ],
                suggested_fix: "Include specific article counts, date ranges, and publication names".to_string(),
                passed: false,
            })
        }
    }
}
```

### 6.3 RoiPresentGate

```rust
// quality/gates/roi_present.rs
use regex::Regex;

pub struct RoiPresentGate {
    /// Patterns indicating ROI calculations
    roi_patterns: Vec<Regex>,
}

impl RoiPresentGate {
    pub fn new() -> Self {
        let patterns = vec![
            r"(?i)\bROI\b",
            r"(?i)\breturn on investment\b",
            r"(?i)\$\d+[KMB]?\s*(savings?|revenue|value)",
            r"(?i)\d+%\s*(increase|decrease|reduction|improvement)",
            r"(?i)\d+x\s*(faster|more|less)",
        ];

        Self {
            roi_patterns: patterns
                .into_iter()
                .map(|p| Regex::new(p).unwrap())
                .collect(),
        }
    }
}

#[async_trait]
impl QualityGate for RoiPresentGate {
    fn name(&self) -> &str {
        "roi_present"
    }

    fn description(&self) -> &str {
        "Validates that ROI calculations are present in final brief"
    }

    async fn check(&self, output: &str) -> Result<ValidationFailure> {
        let mut found_roi = false;

        for pattern in &self.roi_patterns {
            if pattern.is_match(output) {
                found_roi = true;
                break;
            }
        }

        if found_roi {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "Passed".to_string(),
                examples: vec![],
                suggested_fix: String::new(),
                passed: true,
            })
        } else {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "No ROI calculation found in brief".to_string(),
                examples: vec![
                    "Expected: 'ROI: $50K annual savings'".to_string(),
                    "Expected: '3x faster response time'".to_string(),
                ],
                suggested_fix: "Add quantified ROI section with dollar amounts, percentages, or multipliers".to_string(),
                passed: false,
            })
        }
    }
}
```

### 6.4 CaseStudyPresentGate

```rust
// quality/gates/case_study_present.rs
use regex::Regex;

pub struct CaseStudyPresentGate {
    /// Patterns indicating case studies/examples
    case_study_patterns: Vec<Regex>,
}

impl CaseStudyPresentGate {
    pub fn new() -> Self {
        let patterns = vec![
            r"(?i)\b(case study|customer story|success story)\b",
            r"(?i)\b(for example|for instance|specifically)\b",
            r"(?i)\b(helped|enabled|supported)\s+[\w\s]+\s+(achieve|reach|attain)",
            r"(?i)\bcompanies like\b",
        ];

        Self {
            case_study_patterns: patterns
                .into_iter()
                .map(|p| Regex::new(p).unwrap())
                .collect(),
        }
    }
}

#[async_trait]
impl QualityGate for CaseStudyPresentGate {
    fn name(&self) -> &str {
        "case_study_present"
    }

    fn description(&self) -> &str {
        "Ensures at least one specific case study or example is included"
    }

    async fn check(&self, output: &str) -> Result<ValidationFailure> {
        let mut found_case_study = false;

        for pattern in &self.case_study_patterns {
            if pattern.is_match(output) {
                found_case_study = true;
                break;
            }
        }

        if found_case_study {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "Passed".to_string(),
                examples: vec![],
                suggested_fix: String::new(),
                passed: true,
            })
        } else {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::ERROR,
                reason: "No case studies or specific examples found".to_string(),
                examples: vec![
                    "Expected: 'For example, we helped TechCorp reduce crisis response time by 60%'".to_string(),
                ],
                suggested_fix: "Add at least one specific customer example with quantified results".to_string(),
                passed: false,
            })
        }
    }
}
```

### 6.5 ContactValidationGate

```rust
// quality/gates/contact_validation.rs
use regex::Regex;

pub struct ContactValidationGate {
    name_pattern: Regex,
    title_pattern: Regex,
}

impl ContactValidationGate {
    pub fn new() -> Self {
        Self {
            // Matches proper names (capitalized words)
            name_pattern: Regex::new(r"\b[A-Z][a-z]+\s+[A-Z][a-z]+\b").unwrap(),

            // Matches job titles
            title_pattern: Regex::new(
                r"(?i)\b(VP|Vice President|Director|Chief|Head|Manager|Officer)\b"
            ).unwrap(),
        }
    }
}

#[async_trait]
impl QualityGate for ContactValidationGate {
    fn name(&self) -> &str {
        "contact_validation"
    }

    fn description(&self) -> &str {
        "Validates that contact name and title are present (or explicitly marked unavailable)"
    }

    async fn check(&self, output: &str) -> Result<ValidationFailure> {
        let has_name = self.name_pattern.is_match(output);
        let has_title = self.title_pattern.is_match(output);
        let explicitly_unavailable = output.contains("Contact information unavailable")
            || output.contains("Unable to identify");

        if (has_name && has_title) || explicitly_unavailable {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::WARNING, // Warning, not error (contact may be unavailable)
                reason: "Passed".to_string(),
                examples: vec![],
                suggested_fix: String::new(),
                passed: true,
            })
        } else {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::WARNING,
                reason: "No contact name/title found (consider LinkedIn search)".to_string(),
                examples: vec![],
                suggested_fix: "Either include contact name and title, or explicitly state 'Contact information unavailable'".to_string(),
                passed: false,
            })
        }
    }
}
```

### 6.6 MarkdownFormatGate

```rust
// quality/gates/markdown_format.rs
use regex::Regex;

pub struct MarkdownFormatGate {
    header_pattern: Regex,
    section_count_min: usize,
}

impl MarkdownFormatGate {
    pub fn new() -> Self {
        Self {
            header_pattern: Regex::new(r"^#{1,3}\s+.+$").unwrap(),
            section_count_min: 4, // Expect at least 4 sections
        }
    }
}

#[async_trait]
impl QualityGate for MarkdownFormatGate {
    fn name(&self) -> &str {
        "markdown_format"
    }

    fn description(&self) -> &str {
        "Validates markdown structure (headers, sections)"
    }

    async fn check(&self, output: &str) -> Result<ValidationFailure> {
        let header_count = output
            .lines()
            .filter(|line| self.header_pattern.is_match(line))
            .count();

        if header_count >= self.section_count_min {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::WARNING,
                reason: "Passed".to_string(),
                examples: vec![],
                suggested_fix: String::new(),
                passed: true,
            })
        } else {
            Ok(ValidationFailure {
                gate_name: self.name().to_string(),
                severity: GateSeverity::WARNING,
                reason: format!(
                    "Insufficient markdown structure (found {} headers, expected ≥{})",
                    header_count, self.section_count_min
                ),
                examples: vec![],
                suggested_fix: "Ensure output has proper markdown headers (# Header, ## Subheader)".to_string(),
                passed: false,
            })
        }
    }
}
```

---

## 7. Quality Gate Algorithms - Numeric Specifications

**Purpose:** Define exact numeric thresholds and detection algorithms for all quality gates to enable unambiguous implementation and testing.

### 7.1 NoGenericTextGate - Detailed Algorithm

**Detection Method:** Regex pattern matching with penalty scoring

**Generic Keyword List:**
```rust
const GENERIC_KEYWORDS: &[&str] = &[
    "TBD", "TODO", "FIXME", "PLACEHOLDER", "XXX", "YYY", "ZZZ",
    "the company", "this company", "their company", "your company",
    "the organization", "this organization", "the business", "this business",
    "recently", "lately", "in recent times", "in the past", "previously",
    "many", "several", "numerous", "various", "significant", "substantial", "considerable",
    "may help", "might help", "could help", "possibly help", "potentially help",
];
```

**Scoring Algorithm:**
```rust
fn calculate_generic_score(output: &str) -> (i32, Vec<String>) {
    let mut penalty_score = 0;
    let mut examples = Vec::new();

    // Scan for generic keywords (each instance = -10 points)
    for keyword in GENERIC_KEYWORDS {
        let matches: Vec<_> = output.match_indices(keyword).collect();
        penalty_score -= (matches.len() as i32) * 10;
        for (_, matched) in matches.iter().take(5) {
            examples.push(matched.to_string());
        }
    }

    // Scan for bracket placeholders (each = -15 points)
    let bracket_regex = Regex::new(r"\[.*?\]|\{.*?\}|<.*?>").unwrap();
    for mat in bracket_regex.find_iter(output) {
        penalty_score -= 15;
        examples.push(mat.as_str().to_string());
    }

    // Bonus for specific metrics (each number = +2 points, max +20)
    let number_regex = Regex::new(r"\b\d+\b").unwrap();
    let number_count = number_regex.find_iter(output).count();
    penalty_score += std::cmp::min((number_count as i32) * 2, 20);

    (penalty_score, examples)
}
```

**Pass/Fail Threshold:**
- **Score ≥ 0:** PASS (no generic text detected)
- **Score < 0:** FAIL (generic text penalty exceeds specific metric bonus)

**Example Calculations:**
```
Example 1: "TechCorp has 47 articles in Q4 2024"
  - Generic keywords: 0 × -10 = 0
  - Bracket placeholders: 0 × -15 = 0
  - Numbers (47, 2024): 2 × +2 = +4
  - Score: +4 → PASS ✅

Example 2: "The company [Company Name] has significant growth"
  - Generic keywords: 2 ("the company", "significant") × -10 = -20
  - Bracket placeholders: 1 ("[Company Name]") × -15 = -15
  - Numbers: 0 × +2 = 0
  - Score: -35 → FAIL ❌

Example 3: "Recently, many companies saw growth"
  - Generic keywords: 3 ("recently", "many", "companies") × -10 = -30
  - Bracket placeholders: 0 × -15 = 0
  - Numbers: 0 × +2 = 0
  - Score: -30 → FAIL ❌
```

### 7.2 CoverageQuantificationGate - Numeric Threshold

**Detection Method:** Presence validation (boolean check)

**Required Elements:**
1. At least one number (`\b\d+\b`)
2. At least one media metric keyword (`articles?|mentions?|pieces?|stories|posts?|views?|impressions?`)

**Pass Condition:**
```rust
has_numbers && has_media_metric → PASS
otherwise → FAIL
```

**Examples:**
- ✅ "47 articles in Q4 2024" → PASS (has number + media metric)
- ❌ "Significant media coverage" → FAIL (no number)
- ❌ "15 items covered" → FAIL (no media metric keyword)

### 7.3 RoiPresentGate - Numeric Threshold

**Detection Method:** Pattern matching with minimum match requirement

**Required Patterns (minimum 2 matches):**
```rust
const ROI_PATTERNS: &[&str] = &[
    r"(?i)\bROI\b",                                     // Direct ROI mention
    r"(?i)\breturn on investment\b",                    // ROI spelled out
    r"(?i)\$\d+[KMB]?\s*(savings?|revenue|value)",     // Dollar amounts
    r"(?i)\d+%\s*(increase|decrease|reduction|improvement)", // Percentages
    r"(?i)\d+x\s*(faster|more|less)",                  // Multipliers
];
```

**Pass Condition:**
```rust
pattern_match_count >= 2 → PASS
otherwise → FAIL
```

**Examples:**
- ✅ "ROI: $50K savings (3x faster)" → PASS (3 matches: ROI, $50K savings, 3x faster)
- ✅ "25% increase in efficiency, $100K value" → PASS (2 matches: 25% increase, $100K value)
- ❌ "Provides significant value" → FAIL (0 matches)
- ❌ "ROI benefits expected" → FAIL (1 match, need 2)

### 7.4 CaseStudyPresentGate - Numeric Threshold

**Detection Method:** Pattern matching with minimum match requirement

**Required Patterns (minimum 1 match):**
```rust
const CASE_STUDY_PATTERNS: &[&str] = &[
    r"(?i)\b(case study|customer story|success story)\b",
    r"(?i)\b(for example|for instance|specifically)\b",
    r"(?i)\b(helped|enabled|supported)\s+[\w\s]+\s+(achieve|reach|attain)",
    r"(?i)\bcompanies like\b",
];
```

**Pass Condition:**
```rust
pattern_match_count >= 1 → PASS
otherwise → FAIL
```

**Additional Validation (Recommended):**
- If match found, check for specific company name (capitalized proper noun)
- If match found, check for quantified result (number + metric)

**Examples:**
- ✅ "For example, we helped DataCorp achieve 60% faster response" → PASS
- ✅ "Case study: TechCo reduced costs by $100K" → PASS
- ❌ "Our solution works well" → FAIL

### 7.5 ContactValidationGate - Numeric Threshold

**Detection Method:** Pattern matching OR explicit unavailability statement

**Required Patterns (both required for PASS):**
```rust
name_pattern = r"\b[A-Z][a-z]+\s+[A-Z][a-z]+\b"  // Proper name (2 capitalized words)
title_pattern = r"(?i)\b(VP|Vice President|Director|Chief|Head|Manager|Officer)\b"
```

**Pass Conditions:**
```rust
(has_name && has_title) → PASS
OR
contains("Contact information unavailable") → PASS (explicit fallback)
OR
contains("Unable to identify") → PASS (explicit fallback)

otherwise → WARNING (not error, because contact might be legitimately unavailable)
```

**Gate Severity:** WARNING (not ERROR) - allows phase to proceed with logged warning

### 7.6 MarkdownFormatGate - Numeric Threshold

**Detection Method:** Header count validation

**Required Elements:**
```rust
min_headers = 4  // Minimum markdown headers (# or ## or ###)
header_pattern = r"^#{1,3}\s+.+$"  // Lines starting with 1-3 # symbols
```

**Pass Condition:**
```rust
header_count >= 4 → PASS
otherwise → WARNING
```

**Gate Severity:** WARNING (not ERROR) - markdown structure issue doesn't block content quality

**Examples:**
```markdown
# FULLINTEL OPPORTUNITY BRIEF  ← Header 1
## Company Profile              ← Header 2
## Situation Analysis           ← Header 3
## ROI Projection                ← Header 4
Result: 4 headers → PASS ✅
```

### 7.7 Quality Score Calculation - Exact Formula

**Formula:**
```rust
fn calculate_quality_score(failures: &[ValidationFailure], warnings: &[ValidationWarning]) -> u8 {
    let base_score = 100;
    let error_penalty = failures.len() * 25;      // Each ERROR failure = -25 points
    let warning_penalty = warnings.len() * 5;     // Each WARNING = -5 points
    let total_penalty = error_penalty + warning_penalty;

    base_score.saturating_sub(total_penalty as u8)  // Clamp to 0 minimum
}
```

**Examples:**
```
0 failures, 0 warnings → Score = 100 - (0×25) - (0×5) = 100
1 failure, 0 warnings  → Score = 100 - (1×25) - (0×5) = 75
2 failures, 1 warning  → Score = 100 - (2×25) - (1×5) = 45
4 failures, 2 warnings → Score = 100 - (4×25) - (2×5) = 0 (clamped)
```

### 7.8 Phase-Specific Gate Mapping

**Exact gate assignment per phase:**

| Phase | Applied Gates | Blocking Gates | Warning Gates |
|-------|---------------|----------------|---------------|
| **Phase 1** | no_generic_text, contact_validation | no_generic_text | contact_validation |
| **Phase 2** | no_generic_text, coverage_quantification | both | none |
| **Phase 3** | no_generic_text | no_generic_text | none |
| **Phase 4** | no_generic_text, case_study_present | both | none |
| **Phase 5** | no_generic_text, roi_present, markdown_format | no_generic_text, roi_present | markdown_format |

**Blocking Behavior:**
- **ERROR gates:** Phase cannot complete until gate passes or user overrides
- **WARNING gates:** Phase completes but warning logged for review

### 7.9 Implementation Validation

**Test Case Template:**
```rust
#[tokio::test]
async fn test_quality_gate_numeric_threshold() {
    let gate = SpecificGate::new();

    // Test 1: Should PASS
    let pass_input = "Expected passing text with required elements";
    let result = gate.check(pass_input).await.unwrap();
    assert!(result.passed, "Should pass with valid input");

    // Test 2: Should FAIL
    let fail_input = "Text missing required elements";
    let result = gate.check(fail_input).await.unwrap();
    assert!(!result.passed, "Should fail without required elements");

    // Test 3: Edge case (boundary condition)
    let edge_input = "Text with exactly threshold number of elements";
    let result = gate.check(edge_input).await.unwrap();
    assert!(result.passed, "Should pass at exact threshold");
}
```

### 7.10 Traceability to Requirements

| Quality Gate | L0 Requirement | Numeric Threshold | Test Reference |
|--------------|----------------|-------------------|----------------|
| NoGenericTextGate | SR-002 (No generic text) | Score ≥ 0 | TEST-UNIT-4001 |
| CoverageQuantificationGate | SR-002 (Quality standards) | Has number + metric | TEST-UNIT-4002 |
| RoiPresentGate | SR-002 (ROI present) | ≥2 ROI patterns | TEST-UNIT-4003 |
| CaseStudyPresentGate | SR-002 (Case study present) | ≥1 case pattern | TEST-UNIT-4004 |
| ContactValidationGate | Phase 1 output | Has name + title OR unavailable | TEST-UNIT-4005 |
| MarkdownFormatGate | DR-004 (Output format) | ≥4 headers | TEST-UNIT-4006 |

---

## 8. Error Handling

### 7.1 Gate Execution Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum QualityGateError {
    #[error("Gate execution failed: {0}")]
    ExecutionError(String),

    #[error("Invalid output format: {0}")]
    InvalidFormat(String),

    #[error("Gate not registered: {0}")]
    GateNotFound(String),
}
```

### 7.2 Recovery Strategy

| Error Type | Recovery Action |
|-----------|----------------|
| ExecutionError | Log error, treat as validation failure |
| InvalidFormat | Return validation failure with format guidance |
| GateNotFound | Fail immediately (configuration error) |

---

## 8. Testing Requirements

### 8.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_generic_text_gate_detects_placeholders() {
        let gate = NoGenericTextGate::new();

        let bad_output = "The company [Company Name] operates in {industry}.";
        let result = gate.check(bad_output).await.unwrap();
        assert!(!result.passed);
        assert!(result.examples.len() > 0);

        let good_output = "TechCorp operates in enterprise software.";
        let result = gate.check(good_output).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_roi_gate_requires_quantification() {
        let gate = RoiPresentGate::new();

        let bad_output = "Our solution provides significant value.";
        let result = gate.check(bad_output).await.unwrap();
        assert!(!result.passed);

        let good_output = "ROI: $50K annual savings (3x faster response time).";
        let result = gate.check(good_output).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_validator_calculates_quality_score() {
        let mut validator = QualityGateValidator::new();

        let output = "TechCorp recent announcement shows significant growth.";
        let result = validator.validate("phase_5", output).await.unwrap();

        // Should fail no_generic_text and roi_present
        assert!(!result.passed);
        assert!(result.quality_score < 100);
    }
}
```

### 8.2 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_validation_workflow() {
        let mut validator = QualityGateValidator::new();

        // Phase 5 output (final brief)
        let output = r#"
# FULLINTEL OPPORTUNITY BRIEF

## Company Profile
TechCorp Inc. (NASDAQ: TECH) operates in enterprise software with $500M annual revenue.

## Situation Analysis
In Q4 2024, TechCorp received 47 articles across major tech publications following their product launch.

## Communications Intelligence
Current PR team led by Sarah Johnson (VP Communications) manages all media relations.

## Solution Recommendation
For example, we helped DataCorp reduce crisis response time by 60% using our platform.

## ROI Projection
Estimated ROI: $75K annual savings through 3x faster media monitoring.
        "#;

        let result = validator.validate("phase_5", output).await.unwrap();

        assert!(result.passed, "Validation should pass for complete brief");
        assert_eq!(result.failures.len(), 0);
        assert!(result.quality_score >= 90);

        // Check metrics
        assert_eq!(validator.metrics().total_validations, 1);
        assert_eq!(validator.metrics().total_passes, 1);
    }
}
```

---

## 9. Performance Requirements

| Metric | Target | Validation Method |
|--------|--------|------------------|
| **Validation Latency** | < 100ms per phase | Measure across 100 validations |
| **Memory Usage** | < 10MB | Profile with 1000 validations |
| **Regex Compilation** | Once at initialization | Unit test cold start |
| **False Positive Rate** | < 5% | Manual review of 100 outputs |

---

## 10. Configuration

### 10.1 Configurable Thresholds

```rust
// quality/rules.rs
pub struct QualityConfig {
    /// Minimum quality score to pass (0-100)
    pub min_quality_score: u8,

    /// Maximum generic text instances before failure
    pub max_generic_instances: usize,

    /// Minimum markdown header count
    pub min_markdown_headers: usize,

    /// Enable strict mode (warnings become errors)
    pub strict_mode: bool,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            min_quality_score: 75,
            max_generic_instances: 0, // Zero tolerance
            min_markdown_headers: 4,
            strict_mode: false,
        }
    }
}
```

---

## 11. Traceability Matrix

| L2 Interface Requirement | Implementation Element | Validation |
|-------------------------|----------------------|------------|
| ICD-03: ValidationResult schema | `ValidationResult` struct | Unit test deserialization |
| ICD-03: Quality gate validation | `QualityGate` trait | Integration test per gate |
| L1-SAD REQ-SYS-003 | All 6 quality gates | Integration test full workflow |
| L1-SAD MO-002 (90% pass rate) | `QualityMetrics.pass_rate` | Track across 100 sessions |
| L0 SR-002 (No generic text) | `NoGenericTextGate` | Unit test with placeholders |

---

## 12. Future Enhancements

### 12.1 Machine Learning Validation
```rust
// Use LLM to validate output quality
pub async fn validate_with_llm(&self, output: &str) -> Result<ValidationResult> {
    // Send to LLM: "Rate quality 0-100 and identify issues"
    unimplemented!("ML-based validation - post-MVP")
}
```

### 12.2 Custom Gate Registration
```rust
pub fn register_custom_gate(&mut self, gate: Arc<dyn QualityGate>) {
    self.gates.insert(gate.name().to_string(), gate);
}
```

---

**Document Status:** Complete - Ready for L3-CDD-05-StateManager
**Next Document:** L3-CDD-05-StateManager.md
