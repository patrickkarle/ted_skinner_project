#![allow(dead_code)]
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ------------------------------------------------------------------
// Data Structures (Matching the YAML Schema)
// ------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub manifest: ManifestHeader,
    pub schemas: HashMap<String, DataSchema>,
    pub phases: Vec<Phase>,
    pub quality_gates: Vec<QualityGate>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManifestHeader {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    /// Optional label for the research subject input field (e.g., "industry or segment" instead of "company name")
    #[serde(default)]
    pub input_label: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataSchema {
    pub fields: Vec<SchemaField>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SchemaField {
    pub name: String,
    #[serde(default)]
    pub r#enum: Option<Vec<String>>, // 'enum' is a reserved keyword in Rust
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Phase {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub instructions: String,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub output_schema: Option<String>,
    #[serde(default)]
    pub output_target: Option<String>,
    #[serde(default)]
    pub output_format: Option<String>,
    #[serde(default)]
    pub logic_map: Option<HashMap<String, HashMap<String, String>>>,
    /// LLM model to use for this phase (e.g., "claude-3-5-sonnet", "gemini-1.5-flash")
    /// If not specified, defaults to "claude-3-5-sonnet"
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QualityGate {
    pub phase: String,
    pub check: String,
    pub fail_action: String,
}

// ------------------------------------------------------------------
// Implementation
// ------------------------------------------------------------------

impl Manifest {
    /// Load and parse a manifest file from disk
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read manifest file: {:?}", path.as_ref()))?;

        let manifest: Manifest =
            serde_yaml::from_str(&content).with_context(|| "Failed to parse YAML manifest")?;

        Ok(manifest)
    }

    /// Get a specific phase by ID
    pub fn get_phase(&self, id: &str) -> Option<&Phase> {
        self.phases.iter().find(|p| p.id == id)
    }
}

// ------------------------------------------------------------------
// Unit Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_fullintel_manifest() {
        let yaml_content = r#"
manifest:
  id: "PROTO-TEST-001"
  version: "1.0.0"
  name: "Test Protocol"
  description: "Unit test protocol."

schemas:
  TestSchema:
    fields:
      - name: test_field

phases:
  - id: "PHASE-01"
    name: "Context"
    tools: ["search"]
    instructions: "Do research."
    output_schema: "TestSchema"

quality_gates:
  - phase: "PHASE-01"
    check: "Is good?"
    fail_action: "RETRY"
"#;
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", yaml_content).unwrap();

        let manifest = Manifest::load_from_file(file.path()).unwrap();

        assert_eq!(manifest.manifest.id, "PROTO-TEST-001");
        assert_eq!(manifest.phases.len(), 1);
        assert_eq!(manifest.phases[0].tools[0], "search");
        assert_eq!(
            manifest.schemas.get("TestSchema").unwrap().fields[0].name,
            "test_field"
        );
    }
}
