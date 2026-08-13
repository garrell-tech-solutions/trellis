//! What the templates render.
//!
//! Deliberately separate from the store's row types, even where the two
//! currently carry the same single field. A template that renders a
//! `sqlx::FromRow` struct is bound to the query that produced it, and the
//! binding had already started pulling the wrong way: `UntriagedCapture` was
//! documented as "a capture as the inbox view needs it" — persistence
//! described in terms of a page — and `create_capture` was hand-building one
//! for a capture it had just written and never read back, purely because the
//! template demanded that type.
//!
//! A row is what the database returned. A view model is what the page shows.
//! They agree today and will stop agreeing at the next slice: a row needs its
//! capture's id to aim a triage action at, and rendered text is not always a
//! column — but which columns `list_untriaged` selects is still nobody's
//! business but the store's.
//!
//! Nothing here depends on anything. Handlers do the mapping, so this module
//! stays pure data and can be rendered without a database.

/// One capture as the inbox lists it.
pub struct CaptureRow {
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capture_row_carries_the_text_the_page_shows() {
        let row = CaptureRow {
            text: "buy milk".to_string(),
        };
        assert_eq!(row.text, "buy milk");
    }
}
