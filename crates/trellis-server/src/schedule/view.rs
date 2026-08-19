//! What the schedule page renders: one row per placed block, one row per
//! unplaceable task -- both already decided what a reader needs, not what
//! `store` happened to return (`T-templates-take-view-models`).

/// One placed block. `start`/`end` are already formatted as the instants
/// the page shows -- RFC 3339 in UTC, `T-jiff-epoch-millis`'s own format,
/// which is what every acceptance scenario names literally.
pub struct PlacedRow {
    pub text: String,
    pub life_area_name: Option<String>,
    pub start: String,
    pub end: String,
    pub overruns_deadline: bool,
}

/// One task the last generation could not place, and why.
pub struct UnplaceableRow {
    pub text: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placed_row_carries_its_own_fields() {
        let row = PlacedRow {
            text: "write the Q3 deck".to_string(),
            life_area_name: Some("Work".to_string()),
            start: "2026-08-17T09:00:00Z".to_string(),
            end: "2026-08-17T11:00:00Z".to_string(),
            overruns_deadline: false,
        };
        assert_eq!(row.text, "write the Q3 deck");
        assert_eq!(row.life_area_name.as_deref(), Some("Work"));
        assert!(!row.overruns_deadline);
    }

    #[test]
    fn an_unplaceable_row_carries_its_reason() {
        let row = UnplaceableRow {
            text: "rebuild the deck".to_string(),
            reason: "chunk_policy_unsatisfiable".to_string(),
        };
        assert_eq!(row.reason, "chunk_policy_unsatisfiable");
    }
}
