//! What the templates render. A life area as one `<option>` in triage's
//! picker ([`LifeAreaOption`]) and a life area as a row on the management
//! page ([`LifeAreaListItem`]) are different shapes for different readers,
//! not one type reused: the picker never shows a guardrail or a per-row
//! rejection, and binding it to the richer shape would carry fields it
//! never uses (`T-templates-take-view-models`).
//!
//! Neither is [`super::store`]'s row type, even where fields currently
//! match -- the same reasoning `inbox::view` gives for keeping `CaptureRow`
//! separate from its store row: a template bound to a query's row type is
//! bound to that query.

pub struct LifeAreaOption {
    pub id: i64,
    pub name: String,
}

impl From<super::store::LifeAreaRow> for LifeAreaOption {
    fn from(row: super::store::LifeAreaRow) -> Self {
        LifeAreaOption {
            id: row.id,
            name: row.name,
        }
    }
}

/// One guardrail band as the management page shows it: every weekday the
/// owner named in one submission, grouped back into the band they authored
/// (`guardrails-two-bands-03`'s "two bands, one life area" is this
/// grouping's whole point). `id` names one representative row in the group
/// -- the one the remove control posts against, per
/// `store::remove_guardrail_band`'s own contract of taking the whole group
/// with it.
pub struct GuardrailBandGroup {
    pub id: i64,
    pub label: String,
}

/// One life area as the management page's own row: its guardrail state
/// alongside its name, and a slot for the rejection message a failed
/// guardrail save against this row just left (the same shape
/// `inbox::view::CaptureRow` carries for a failed triage).
pub struct LifeAreaListItem {
    pub id: i64,
    pub name: String,
    pub pool_only: bool,
    pub bands: Vec<GuardrailBandGroup>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_life_area_option_carries_its_id_and_name() {
        let option = LifeAreaOption {
            id: 3,
            name: "Learning".to_string(),
        };
        assert_eq!(option.id, 3);
        assert_eq!(option.name, "Learning");
    }

    #[test]
    fn a_life_area_list_item_carries_its_guardrail_state_and_error_slot() {
        let item = LifeAreaListItem {
            id: 3,
            name: "Learning".to_string(),
            pool_only: false,
            bands: vec![GuardrailBandGroup {
                id: 7,
                label: "Mon, Wed, Fri 06:00-07:00".to_string(),
            }],
            error: Some("the band overlaps one this life area already has".to_string()),
        };
        assert_eq!(item.id, 3);
        assert_eq!(item.name, "Learning");
        assert!(!item.pool_only);
        assert_eq!(item.bands.len(), 1);
        assert_eq!(item.bands[0].label, "Mon, Wed, Fri 06:00-07:00");
        assert!(item.error.is_some());
    }
}
