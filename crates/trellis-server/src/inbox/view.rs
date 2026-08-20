//! What the templates render.
//!
//! Deliberately separate from [`super::store`]'s row types, even where the
//! two currently carry the same single field. A template that renders a
//! `sqlx::FromRow` struct is bound to the query that produced it, and the
//! binding had already started pulling the wrong way: `UntriagedCapture` was
//! documented as "a capture as the inbox view needs it" — persistence
//! described in terms of a page — and `create_capture` was hand-building one
//! for a capture it had just written and never read back, purely because the
//! template demanded that type.
//!
//! A row is what the database returned. A view model is what the page shows.
//! They no longer agree: a capture row now carries the id the triage-from-page
//! slice aims its controls at, and an in-flight rejection's message — neither
//! of which is a column `list_untriaged` selects.
//!
//! Nothing here depends on anything. Handlers do the mapping, so this module
//! stays pure data and can be rendered without a database.

/// One capture as the inbox lists it, with room for the rejection message a
/// failed triage attempt against it leaves behind.
pub struct CaptureRow {
    pub id: i64,
    pub text: String,
    /// The capture's own context tag (#82), already resolved -- `None` for
    /// one that carries none.
    pub context_tag: Option<String>,
    pub error: Option<String>,
}

/// One task as the task list shows it. `life_area` is `None` for a task
/// with no life area — either written before the life-areas migration
/// landed, or triaged since `T-life-area-required-at-triage` was superseded
/// (#82): the two are indistinguishable and both mean "no life area", which
/// is exactly what `D-manual-triage-until-llm` asks for. `context_tag` is
/// read through the capture the task came from -- the tag lives there, not
/// on the task (`features/context_tags.feature`'s own "one fact, one row").
pub struct TaskRow {
    pub kind: String,
    pub text: String,
    pub life_area: Option<String>,
    pub context_tag: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capture_row_carries_its_id_text_tag_and_error() {
        let row = CaptureRow {
            id: 7,
            text: "buy milk".to_string(),
            context_tag: Some("@supermarket".to_string()),
            error: Some("deadline is required".to_string()),
        };
        assert_eq!(row.id, 7);
        assert_eq!(row.text, "buy milk");
        assert_eq!(row.context_tag.as_deref(), Some("@supermarket"));
        assert_eq!(row.error.as_deref(), Some("deadline is required"));
    }

    #[test]
    fn a_task_row_carries_its_kind_text_life_area_and_tag() {
        let row = TaskRow {
            kind: "pool".to_string(),
            text: "buy milk".to_string(),
            life_area: Some("Home".to_string()),
            context_tag: Some("@supermarket".to_string()),
        };
        assert_eq!(row.kind, "pool");
        assert_eq!(row.text, "buy milk");
        assert_eq!(row.context_tag.as_deref(), Some("@supermarket"));
        assert_eq!(row.life_area.as_deref(), Some("Home"));
    }
}
