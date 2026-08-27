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
///
/// `committed_open`/`quota_open` are the view's own reading of the store's
/// `shown_kind` text column (#119) -- a boolean per kind rather than the raw
/// string, the same shift `row.past`/`life_area.pool_only` already made
/// elsewhere so a template compares a flag, never a literal. Mutually
/// exclusive by construction: [`set_shown_kind`](super::store::set_shown_kind)
/// only ever writes one of the two kinds it's given.
pub struct CaptureRow {
    pub id: i64,
    pub text: String,
    pub context_tag: Option<String>,
    pub error: Option<String>,
    pub committed_open: bool,
    pub quota_open: bool,
    /// `Some("Pool · @homedepot")` once triaged — the row's kind buttons
    /// give way to this line, restyled rather than removed (#140,
    /// `inbox-view-triaged-row-stays-04`: there are four screens for a
    /// triaged task to live on now, but no feedback at all if this one
    /// vanishes). `None` while the capture is still untriaged.
    pub meta: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn untriaged_row() -> CaptureRow {
        CaptureRow {
            id: 7,
            text: "buy milk".to_string(),
            context_tag: Some("@homedepot".to_string()),
            error: Some("deadline is required".to_string()),
            committed_open: false,
            quota_open: false,
            meta: None,
        }
    }

    #[test]
    fn a_capture_row_carries_its_id_text_tag_and_error() {
        let row = untriaged_row();
        assert_eq!(row.id, 7);
        assert_eq!(row.text, "buy milk");
        assert_eq!(row.context_tag.as_deref(), Some("@homedepot"));
        assert_eq!(row.error.as_deref(), Some("deadline is required"));
    }

    #[test]
    fn a_capture_row_carries_which_kinds_panel_is_open() {
        let row = CaptureRow {
            committed_open: true,
            ..untriaged_row()
        };
        assert!(row.committed_open);
        assert!(!row.quota_open);
    }

    #[test]
    fn a_triaged_capture_row_carries_what_it_became_and_no_kind_panel() {
        let row = CaptureRow {
            meta: Some("Pool · @homedepot".to_string()),
            ..untriaged_row()
        };
        assert_eq!(row.meta.as_deref(), Some("Pool · @homedepot"));
    }
}
