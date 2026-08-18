//! What the capacity page renders: one row per active life area, already
//! decided which of the two shapes it takes (`T-templates-take-view-models`
//! -- the template does not compute `never_scheduled` from a guardrail, it
//! reads a bool this module already resolved).

/// One life area's row. `never_scheduled` life areas carry no other
/// meaningful field -- `T-guardrail-well-formedness`: opted out of being
/// scheduled is not the same as full, so the page shows neither a number
/// nor a warning for one.
pub struct CapacityRow {
    pub id: i64,
    pub name: String,
    pub never_scheduled: bool,
    pub needed_hours: f64,
    pub available_hours: f64,
    pub percent_used: i64,
    pub over_hours: Option<f64>,
    pub unestimated_committed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_never_scheduled_row_carries_its_id_and_name() {
        let row = CapacityRow {
            id: 2,
            name: "Fitness".to_string(),
            never_scheduled: true,
            needed_hours: 0.0,
            available_hours: 0.0,
            percent_used: 0,
            over_hours: None,
            unestimated_committed: 0,
        };
        assert_eq!(row.id, 2);
        assert_eq!(row.name, "Fitness");
        assert!(row.never_scheduled);
    }

    #[test]
    fn a_measured_row_carries_its_own_numbers() {
        let row = CapacityRow {
            id: 2,
            name: "Fitness".to_string(),
            never_scheduled: false,
            needed_hours: 5.0,
            available_hours: 4.0,
            percent_used: 125,
            over_hours: Some(1.0),
            unestimated_committed: 1,
        };
        assert_eq!(row.needed_hours, 5.0);
        assert_eq!(row.available_hours, 4.0);
        assert_eq!(row.percent_used, 125);
        assert_eq!(row.over_hours, Some(1.0));
        assert_eq!(row.unestimated_committed, 1);
    }
}
