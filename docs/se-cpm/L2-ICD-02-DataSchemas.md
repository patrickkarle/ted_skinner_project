# L2-ICD-02: Data Schemas Interface Control Document

**Document ID:** L2-ICD-DATA-001
**Interface Name:** Cross-Phase Data Contracts
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L1-SAD-FULLINTEL-001
**Traceability:** L1-SAD-1.1 MO-001, MO-002 (Workflow execution, Quality standardization)

---

## 1. Interface Overview

### 1.1 Purpose
Defines the data contracts passed between the 5 workflow phases, ensuring type safety and structural consistency across the research pipeline.

### 1.2 Data Flow
```
Phase 1 → CompanyProfile → Phase 2
Phase 2 → SituationAnalysis → Phases 3, 4
Phase 3 → PainPointsList + ContactInfo → Phase 5
Phase 4 → SolutionPackage → Phase 5
Phase 5 → MarkdownBrief → Export
```

### 1.3 Serialization Format
- **In-Memory (Rust):** Native Rust structs with serde serialization
- **Storage (SQLite):** JSON TEXT columns
- **IPC (Tauri):** JSON over message passing
- **LLM Context:** JSON string in prompts

---

## 2. Phase 1 Output: CompanyProfile

### 2.1 Schema Definition

**Rust Struct:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyProfile {
    pub company_name: String,
    pub industry_classification: String,
    pub revenue_tier: RevenueTier,
    pub employee_count_range: Option<EmployeeRange>,
    pub geographic_footprint: Vec<String>,
    pub headquarters_location: Option<String>,
    pub communications_leader_name: Option<String>,
    pub communications_leader_title: Option<String>,
    pub founded_year: Option<u16>,
    pub public_or_private: Option<CompanyType>,
    pub stock_ticker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevenueTier {
    Under10M,
    From10MTo50M,
    From50MTo100M,
    From100MTo500M,
    From500MTo1B,
    Over1B,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmployeeRange {
    Under50,
    From50To200,
    From200To500,
    From500To1000,
    From1000To5000,
    Over5000,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompanyType {
    Public,
    Private,
    NonProfit,
    Government,
}
```

**TypeScript Interface:**
```typescript
interface CompanyProfile {
  company_name: string;
  industry_classification: string;
  revenue_tier: 'Under10M' | 'From10MTo50M' | 'From50MTo100M' |
                'From100MTo500M' | 'From500MTo1B' | 'Over1B' | 'Unknown';
  employee_count_range?: 'Under50' | 'From50To200' | 'From200To500' |
                         'From500To1000' | 'From1000To5000' | 'Over5000' | 'Unknown';
  geographic_footprint: string[];
  headquarters_location?: string;
  communications_leader_name?: string;
  communications_leader_title?: string;
  founded_year?: number;
  public_or_private?: 'Public' | 'Private' | 'NonProfit' | 'Government';
  stock_ticker?: string;
}
```

### 2.2 Example Instance
```json
{
  "company_name": "TechCorp Industries",
  "industry_classification": "Enterprise Software / SaaS",
  "revenue_tier": "From100MTo500M",
  "employee_count_range": "From500To1000",
  "geographic_footprint": ["United States", "Canada", "UK"],
  "headquarters_location": "San Francisco, CA",
  "communications_leader_name": "Jane Smith",
  "communications_leader_title": "VP of Corporate Communications",
  "founded_year": 2010,
  "public_or_private": "Private",
  "stock_ticker": null
}
```

### 2.3 Validation Rules
| Field | Constraint | Error Message |
|-------|-----------|---------------|
| company_name | Required, 1-200 chars | "Company name required" |
| industry_classification | Required, 1-100 chars | "Industry classification required" |
| revenue_tier | Must be valid enum | "Invalid revenue tier" |
| geographic_footprint | Min 1 location | "At least one location required" |
| founded_year | If present: 1800-2025 | "Invalid founding year" |

---

## 3. Phase 2 Output: SituationAnalysis

### 3.1 Schema Definition

**Rust Struct:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SituationAnalysis {
    pub scenario_type: ScenarioType,
    pub coverage_volume: CoverageMetrics,
    pub coverage_momentum: CoverageMomentum,
    pub urgency_level: UrgencyLevel,
    pub key_events: Vec<KeyEvent>,
    pub sentiment_summary: SentimentSummary,
    pub stakeholder_concerns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioType {
    Crisis,           // Negative event requiring damage control
    Launch,           // Product/service launch
    MergersAcquisitions, // M&A activity
    Regulatory,       // Regulatory scrutiny or changes
    Competitive,      // Competitive threat or market shift
    ExecutiveChange,  // Leadership transition
    Other(String),    // Custom scenario type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub total_articles: u32,        // Quantified count
    pub timeframe_days: u8,          // Lookback period
    pub top_sources: Vec<String>,    // Publication names
    pub article_headlines: Vec<String>, // Sample headlines
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageMomentum {
    Increasing,  // Coverage growing over time
    Stable,      // Consistent coverage
    Declining,   // Coverage decreasing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    High,    // Immediate action required
    Medium,  // Moderate priority
    Low,     // Informational, low priority
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub date: String,           // ISO-8601 date
    pub headline: String,       // Article headline
    pub source: String,         // Publication name
    pub summary: String,        // Brief summary
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentSummary {
    pub overall_sentiment: Sentiment,
    pub positive_themes: Vec<String>,
    pub negative_themes: Vec<String>,
    pub neutral_themes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
    Mixed,
}
```

**TypeScript Interface:**
```typescript
interface SituationAnalysis {
  scenario_type: 'Crisis' | 'Launch' | 'MergersAcquisitions' |
                 'Regulatory' | 'Competitive' | 'ExecutiveChange' | { Other: string };
  coverage_volume: {
    total_articles: number;
    timeframe_days: number;
    top_sources: string[];
    article_headlines: string[];
  };
  coverage_momentum: 'Increasing' | 'Stable' | 'Declining';
  urgency_level: 'High' | 'Medium' | 'Low';
  key_events: Array<{
    date: string;
    headline: string;
    source: string;
    summary: string;
  }>;
  sentiment_summary: {
    overall_sentiment: 'Positive' | 'Negative' | 'Neutral' | 'Mixed';
    positive_themes: string[];
    negative_themes: string[];
    neutral_themes: string[];
  };
  stakeholder_concerns: string[];
}
```

### 3.2 Quality Gate Validation
```rust
impl SituationAnalysis {
    /// Validates that coverage volume is properly quantified
    pub fn validate_coverage_quantification(&self) -> Result<(), String> {
        if self.coverage_volume.total_articles == 0 {
            return Err("Coverage volume not quantified - must have article count".to_string());
        }

        if self.coverage_volume.article_headlines.is_empty() {
            return Err("No article headlines provided".to_string());
        }

        Ok(())
    }
}
```

### 3.3 Example Instance
```json
{
  "scenario_type": "Crisis",
  "coverage_volume": {
    "total_articles": 47,
    "timeframe_days": 14,
    "top_sources": ["TechCrunch", "Reuters", "Bloomberg"],
    "article_headlines": [
      "TechCorp faces data breach affecting 2M users",
      "Customers report unauthorized access to accounts",
      "CEO issues statement on security incident"
    ]
  },
  "coverage_momentum": "Increasing",
  "urgency_level": "High",
  "key_events": [
    {
      "date": "2025-11-15",
      "headline": "TechCorp announces data breach",
      "source": "TechCrunch",
      "summary": "Company disclosed unauthorized access to customer database"
    }
  ],
  "sentiment_summary": {
    "overall_sentiment": "Negative",
    "positive_themes": ["Transparent communication", "Quick response"],
    "negative_themes": ["Security concerns", "Customer trust issues"],
    "neutral_themes": ["Industry-wide problem"]
  },
  "stakeholder_concerns": [
    "Data privacy and security",
    "Customer trust restoration",
    "Regulatory compliance"
  ]
}
```

---

## 4. Phase 3 Output: CommunicationsIntelligence

### 4.1 Schema Definition

**Rust Struct:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationsIntelligence {
    pub pain_points: Vec<PainPoint>,
    pub contact_information: Option<ContactInfo>,
    pub team_structure: Option<String>,
    pub recent_initiatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PainPoint {
    pub pain_point: String,        // Specific pain point description
    pub severity: Severity,         // How critical
    pub related_scenario: String,   // Ties back to scenario_type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub name: String,
    pub title: String,
    pub linkedin_url: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}
```

### 4.2 Example Instance
```json
{
  "pain_points": [
    {
      "pain_point": "Managing crisis communications across 24/7 news cycle",
      "severity": "Critical",
      "related_scenario": "Crisis response to data breach"
    },
    {
      "pain_point": "Coordinating messaging across legal, PR, and executive teams",
      "severity": "High",
      "related_scenario": "Internal alignment during crisis"
    }
  ],
  "contact_information": {
    "name": "Jane Smith",
    "title": "VP of Corporate Communications",
    "linkedin_url": "https://linkedin.com/in/janesmith",
    "email": null,
    "phone": null
  },
  "team_structure": "15-person communications team with dedicated crisis response unit",
  "recent_initiatives": [
    "Launched new crisis communication playbook in Q3 2024",
    "Implemented media monitoring platform"
  ]
}
```

---

## 5. Phase 4 Output: SolutionPackage

### 5.1 Schema Definition

**Rust Struct:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionPackage {
    pub recommended_solution: String,        // Fullintel service package
    pub solution_rationale: String,          // Why this solution fits
    pub case_study: CaseStudy,               // Relevant client example
    pub roi_projection: ROIProjection,       // Expected value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseStudy {
    pub client_name: String,             // Real client (or anonymized)
    pub scenario_match: ScenarioType,    // Must match current scenario
    pub challenge_faced: String,         // What problem they had
    pub solution_implemented: String,    // What Fullintel did
    pub results_achieved: Vec<String>,   // Quantified outcomes
    pub timeframe: String,               // How long to see results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROIProjection {
    pub cost_estimate: CostEstimate,
    pub value_drivers: Vec<ValueDriver>,
    pub payback_period: String,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub range_low: f64,
    pub range_high: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueDriver {
    pub driver: String,
    pub quantified_impact: String,
}
```

### 5.2 Quality Gate Validation
```rust
impl SolutionPackage {
    /// Validates that case study is specific and relevant
    pub fn validate_case_study(&self) -> Result<(), String> {
        if self.case_study.client_name.contains("placeholder") ||
           self.case_study.client_name.contains("[") {
            return Err("Case study contains placeholder text".to_string());
        }

        if self.case_study.results_achieved.is_empty() {
            return Err("Case study missing quantified results".to_string());
        }

        Ok(())
    }

    /// Validates that ROI projection is present and specific
    pub fn validate_roi(&self) -> Result<(), String> {
        if self.roi_projection.value_drivers.is_empty() {
            return Err("ROI projection missing value drivers".to_string());
        }

        for driver in &self.roi_projection.value_drivers {
            if !driver.quantified_impact.chars().any(|c| c.is_digit(10)) {
                return Err(format!("Value driver '{}' not quantified", driver.driver));
            }
        }

        Ok(())
    }
}
```

### 5.3 Example Instance
```json
{
  "recommended_solution": "Crisis Communications Response Package",
  "solution_rationale": "TechCorp faces high-urgency crisis with 47 articles in 14 days and increasing momentum. Requires 24/7 media monitoring, rapid response capabilities, and executive spokesperson training.",
  "case_study": {
    "client_name": "DataSafe Inc",
    "scenario_match": "Crisis",
    "challenge_faced": "Faced similar data breach with 63 articles in first week, negative sentiment at 78%",
    "solution_implemented": "Fullintel deployed crisis response team with 24/7 monitoring, drafted 15 executive statements, coordinated 8 media interviews",
    "results_achieved": [
      "Reduced negative coverage by 42% in 30 days",
      "Achieved 89% message consistency across 25 media outlets",
      "Restored customer trust score from 34% to 67% in 90 days"
    ],
    "timeframe": "Visible impact within 14 days, full crisis resolution in 90 days"
  },
  "roi_projection": {
    "cost_estimate": {
      "range_low": 75000.0,
      "range_high": 125000.0,
      "currency": "USD"
    },
    "value_drivers": [
      {
        "driver": "Customer retention",
        "quantified_impact": "Preventing 15% customer churn saves $2.3M in annual recurring revenue"
      },
      {
        "driver": "Brand value protection",
        "quantified_impact": "Maintaining brand trust preserves estimated $5M in enterprise sales pipeline"
      }
    ],
    "payback_period": "Within 60 days based on prevented customer churn",
    "assumptions": [
      "Current customer churn rate of 8% increases to 23% without intervention",
      "Average customer lifetime value of $45,000"
    ]
  }
}
```

---

## 6. Phase 5 Output: MarkdownBrief

### 6.1 Schema Definition

**Rust Struct:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownBrief {
    pub raw_markdown: String,           // Full markdown content
    pub metadata: BriefMetadata,        // Metadata about the brief
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefMetadata {
    pub generated_at: String,           // ISO-8601 timestamp
    pub word_count: usize,
    pub character_count: usize,
    pub sections_present: Vec<String>,  // Which sections included
    pub quality_gate_passed: bool,      // All gates passed
}
```

### 6.2 Required Sections
```markdown
# FULLINTEL OPPORTUNITY BRIEF
## Target Company: [CompanyProfile.company_name]

### Executive Summary
[2-3 sentences synthesizing the opportunity]

### Situation Analysis
- **Scenario Type:** [SituationAnalysis.scenario_type]
- **Coverage Volume:** [SituationAnalysis.coverage_volume.total_articles] articles in last [timeframe_days] days
- **Momentum:** [coverage_momentum]
- **Urgency Level:** [urgency_level]

[Narrative description of the situation]

### Pain Points
[Bulleted list from CommunicationsIntelligence.pain_points]

### Recommended Solution
**[SolutionPackage.recommended_solution]**

[Solution rationale and implementation approach]

### ROI Projection
[Quantified value drivers with specific numbers]

**Investment:** $[range_low] - $[range_high]
**Payback Period:** [payback_period]

### Relevant Case Study: [CaseStudy.client_name]
**Challenge:** [challenge_faced]
**Solution:** [solution_implemented]
**Results:**
[Bulleted list of quantified outcomes]

### Next Steps
1. [Specific actionable recommendation]
2. [Specific actionable recommendation]
3. [Specific actionable recommendation]

---
*Generated by Fullintel Sales Intelligence Generator on [timestamp]*
```

### 6.3 Quality Gate Validation
```rust
impl MarkdownBrief {
    /// Comprehensive quality validation
    pub fn validate_quality(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // No generic/placeholder text
        let generic_terms = ["placeholder", "[insert", "TODO", "TBD", "generic"];
        for term in &generic_terms {
            if self.raw_markdown.to_lowercase().contains(term) {
                errors.push(format!("Contains generic text: '{}'", term));
            }
        }

        // ROI calculations present
        if !self.raw_markdown.contains('$') && !self.raw_markdown.contains("USD") {
            errors.push("No dollar amounts found in ROI projection".to_string());
        }

        // Case study present
        if !self.raw_markdown.contains("Case Study") {
            errors.push("Case study section missing".to_string());
        }

        // Required sections present
        let required_sections = [
            "Executive Summary",
            "Situation Analysis",
            "Pain Points",
            "Recommended Solution",
            "ROI Projection",
            "Case Study",
            "Next Steps"
        ];

        for section in &required_sections {
            if !self.raw_markdown.contains(section) {
                errors.push(format!("Missing required section: {}", section));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

---

## 7. Context Accumulation Pattern

### 7.1 Workflow Context Structure
```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub target_company: String,
    pub phase_outputs: HashMap<String, serde_json::Value>,
    pub metadata: WorkflowMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub session_id: String,
    pub started_at: String,
    pub current_phase: String,
    pub completed_phases: Vec<String>,
}
```

### 7.2 Context Building Example
```rust
// Phase 1 completes
context.phase_outputs.insert(
    "CompanyProfile".to_string(),
    serde_json::to_value(company_profile)?
);

// Phase 2 uses Phase 1 output
let company_profile: CompanyProfile = serde_json::from_value(
    context.phase_outputs.get("CompanyProfile")
        .ok_or("Missing CompanyProfile")?
        .clone()
)?;

// Phase 5 uses ALL previous outputs
let company_profile: CompanyProfile = /* ... */;
let situation: SituationAnalysis = /* ... */;
let comms_intel: CommunicationsIntelligence = /* ... */;
let solution: SolutionPackage = /* ... */;

// Combine into final brief
let brief = generate_brief(company_profile, situation, comms_intel, solution)?;
```

---

## 8. Traceability Matrix

| L1 Requirement | Data Schema | Validation | Quality Gate |
|----------------|-------------|------------|--------------|
| MO-001: Time < 5 min | All schemas optimized for fast serialization | Performance test | N/A |
| MO-002: Quality standardization | Quality gate validators | Unit tests | Phase 2, Phase 5 |
| SR-002: No generic text | MarkdownBrief validation | Regex checks | Phase 5 |
| SR-002: ROI present | ROIProjection required | Validation method | Phase 4, Phase 5 |
| SR-002: Case study | CaseStudy required | Validation method | Phase 4, Phase 5 |

---

**Document Status:** Complete
**Next Document:** L2-ICD-03-ComponentInterfaces.md
