//! Acceptance runtime (APS acceptance-generator.md "Acceptance Runtime
//! Contract"): expands scenario executions, prepends background steps,
//! resolves placeholders, and routes each step to a project step handler.

use crate::ir::{Feature, Step};
use crate::steps;
use crate::world::World;
use std::collections::BTreeMap;

pub async fn run_feature(feature: &Feature) -> Vec<(String, Result<(), String>)> {
    let mut results = Vec::new();
    for scenario in &feature.scenarios {
        let examples: Vec<BTreeMap<String, String>> = if scenario.examples.is_empty() {
            vec![BTreeMap::new()]
        } else {
            scenario.examples.clone()
        };
        for (index, example) in examples.iter().enumerate() {
            let execution_name = format!("{}/example_{}", scenario.name, index + 1);
            let all_steps: Vec<Step> = feature
                .background
                .iter()
                .cloned()
                .chain(scenario.steps.iter().cloned())
                .collect();
            let outcome = run_execution(&all_steps, example).await;
            results.push((execution_name, outcome));
        }
    }
    results
}

pub async fn run_execution(
    all_steps: &[Step],
    example: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut world = World::new();
    for step in all_steps {
        steps::dispatch(&mut world, step, example).await?;
    }
    Ok(())
}
