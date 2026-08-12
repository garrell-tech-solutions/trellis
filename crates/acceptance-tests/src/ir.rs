//! JSON IR shape produced by `bb gherkin-parser`. See
//! ~/.cache/swarmforge/aps-pipeline/parser-spec.md for the canonical spec.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct Feature {
    pub name: String,
    #[serde(default)]
    pub background: Vec<Step>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub examples: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub keyword: String,
    pub text: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub parameters: Vec<String>,
}
