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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Scenario;

    fn undefined_step(text: &str) -> Step {
        // No step handler matches this text, so `steps::dispatch` deterministically
        // errors without needing real IO -- an ideal fixture for exercising runtime
        // control flow in isolation from the step definitions themselves.
        Step {
            keyword: "Given".to_string(),
            text: text.to_string(),
            parameters: vec![],
        }
    }

    #[tokio::test]
    async fn run_execution_succeeds_with_no_steps() {
        assert_eq!(run_execution(&[], &BTreeMap::new()).await, Ok(()));
    }

    #[tokio::test]
    async fn run_execution_propagates_a_step_error() {
        let steps = vec![undefined_step("this step is not defined anywhere")];
        assert!(run_execution(&steps, &BTreeMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn run_feature_produces_one_named_result_per_scenario_example() {
        let feature = Feature {
            name: "test feature".to_string(),
            background: vec![],
            scenarios: vec![
                Scenario {
                    name: "scenario a".to_string(),
                    steps: vec![undefined_step("undefined step one")],
                    examples: vec![BTreeMap::new(), BTreeMap::new()],
                },
                Scenario {
                    name: "scenario b".to_string(),
                    steps: vec![undefined_step("undefined step two")],
                    examples: vec![],
                },
            ],
        };
        let results = run_feature(&feature).await;
        let names: Vec<&str> = results.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "scenario a/example_1",
                "scenario a/example_2",
                "scenario b/example_1"
            ]
        );
        assert!(results.iter().all(|(_, outcome)| outcome.is_err()));
    }

    #[tokio::test]
    async fn run_feature_prepends_background_steps_to_each_scenario() {
        let feature = Feature {
            name: "test feature".to_string(),
            background: vec![undefined_step("undefined background step")],
            scenarios: vec![Scenario {
                name: "scenario".to_string(),
                steps: vec![],
                examples: vec![],
            }],
        };
        let results = run_feature(&feature).await;
        assert_eq!(results.len(), 1);
        assert!(
            results[0].1.is_err(),
            "a failing background step should fail the scenario"
        );
    }
}
