//! What the templates render: a life area as a row in the management list
//! and as one `<option>` in triage's picker share the same shape, so one
//! type serves both (`T-templates-take-view-models`).
//!
//! Deliberately its own type rather than [`super::store::LifeAreaRow`] even
//! though the fields currently match exactly -- the same reasoning
//! `inbox::view` gives for keeping `CaptureRow` separate from its store row:
//! a template bound to a query's row type is bound to that query, and the
//! two have already started to differ in purpose even where they have not
//! yet differed in shape.

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
}
