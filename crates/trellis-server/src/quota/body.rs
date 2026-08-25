//! The `#quota-body` fragment: the quota screen's own content, swappable on
//! its own -- the same shape `pool::body` and `committed::body` take.

use crate::platform::response::{render_template, write_failed};
use crate::quota::view::{self, QuotaRowView};
use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// `"+ Define a new quota"` — the define form's own submit button, unless a
/// warning changes it (`quota-screen-similar-name-warns-07`'s own "Create
/// anyway").
pub(super) const DEFAULT_BUTTON_LABEL: &str = "+ Define a new quota";

/// The define form's own state for this render: what the submitter typed
/// (echoed back so a rejection does not clear the fields), and any warning
/// the name drew. A fresh `GET /quota` renders [`DefineFormView::default`].
pub(super) struct DefineFormView {
    pub(super) pending_name: String,
    pub(super) pending_hours: String,
    pub(super) warning: Option<String>,
    pub(super) button_label: &'static str,
    /// Present only once a *similar* (not exact) name has been warned about
    /// — the hidden field the button's own resubmission carries, so a
    /// repeat of this exact name creates it
    /// (`quota-screen-similar-name-warns-07`). `None` for a fresh form or an
    /// exact-match refusal, neither of which is bypassable.
    pub(super) confirm_name: Option<String>,
}

impl Default for DefineFormView {
    fn default() -> Self {
        DefineFormView {
            pending_name: String::new(),
            pending_hours: String::new(),
            warning: None,
            button_label: DEFAULT_BUTTON_LABEL,
            confirm_name: None,
        }
    }
}

/// The `#quota-body` fragment on its own -- what a rejected definition
/// swaps in with its warning attached.
#[derive(Template)]
#[template(path = "quota_body.html")]
pub(super) struct QuotaBodyTemplate {
    pub(super) meta: String,
    pub(super) empty: bool,
    pub(super) quotas: Vec<QuotaRowView>,
    pub(super) pending_name: String,
    pub(super) pending_hours: String,
    pub(super) warning: Option<String>,
    pub(super) button_label: &'static str,
    pub(super) confirm_name: Option<String>,
}

/// Fetches the current quota list and builds the `#quota-body` fragment.
/// Shared by the quota page (a fresh [`DefineFormView::default`]) and
/// [`respond`] (whatever the definition attempt left behind).
pub(super) async fn build(
    pool: &SqlitePool,
    form: DefineFormView,
) -> Result<QuotaBodyTemplate, sqlx::Error> {
    let rows = super::store::list_quotas(pool).await?;
    let built = view::build(rows);
    Ok(QuotaBodyTemplate {
        meta: built.meta,
        empty: built.empty,
        quotas: built.quotas,
        pending_name: form.pending_name,
        pending_hours: form.pending_hours,
        warning: form.warning,
        button_label: form.button_label,
        confirm_name: form.confirm_name,
    })
}

/// The `#quota-body` fragment, re-rendered from current state --
/// `T-forms-swap-one-fragment`'s response contract -- at `status`, which is
/// `200`/`201` for an accepted definition and `422`
/// (`T-422-is-product-wide`) for a rejected one.
pub(super) async fn respond(
    pool: &SqlitePool,
    status: StatusCode,
    form: DefineFormView,
) -> Result<Response, StatusCode> {
    let body = build(pool, form).await.map_err(write_failed)?;
    Ok(render_template(status, &body))
}
