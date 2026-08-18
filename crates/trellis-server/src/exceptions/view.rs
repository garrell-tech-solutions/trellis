//! What the exceptions list on the free time page renders -- not
//! [`super::store::ExceptionRow`], the same reasoning `life_areas::view`
//! gives for keeping its own row-shaped types apart from the store's
//! (`T-templates-take-view-models`): a row carries a nullable id and raw
//! date text, and a template should never have to resolve either itself.

/// One exception as the page shows it: its scope already resolved to
/// words ("All life areas", or the life area's own name -- scope is shown,
/// never implied, `exceptions-scope-is-visible-03`), and its dates already
/// formatted.
pub struct ExceptionListItem {
    pub id: i64,
    pub scope: String,
    pub start: String,
    pub end: String,
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exception_list_item_carries_its_resolved_scope_and_dates() {
        let item = ExceptionListItem {
            id: 3,
            scope: "Work".to_string(),
            start: "2026-08-24".to_string(),
            end: "2026-08-28".to_string(),
            label: "vacation".to_string(),
        };
        assert_eq!(item.id, 3);
        assert_eq!(item.scope, "Work");
        assert_eq!(item.start, "2026-08-24");
        assert_eq!(item.end, "2026-08-28");
        assert_eq!(item.label, "vacation");
    }
}
