//! The header's own vocabulary: which pages exist, and what a page's link
//! to itself and to its siblings looks like (`T-nav-is-the-site-map`, the
//! header is the route table).
//!
//! Four pages — Capture, Pool (#92), Quota (#93) and Committed (#94) — the
//! full set `D-four-screens` draws. `Quota` was the last dead link this
//! module carried the comment for; [`ALL`] now names every page
//! [`platform::app`] can route to.

/// A page the header can link to. Add a variant only alongside the route
/// it names — [`ALL`] and [`links`] are what keep the header from ever
/// offering a page `platform::app` cannot serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Capture,
    Pool,
    Committed,
    Quota,
}

/// Every page that exists, in the order the header lists them --
/// `committed-screen-tabs-06`'s own order, `Quota` last.
pub(crate) const ALL: [Page; 4] = [Page::Capture, Page::Pool, Page::Committed, Page::Quota];

impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Capture => "Capture",
            Page::Pool => "Pool",
            Page::Committed => "Committed",
            Page::Quota => "Quota",
        }
    }

    pub(crate) fn path(self) -> &'static str {
        match self {
            Page::Capture => "/",
            Page::Pool => "/pool",
            Page::Committed => "/committed",
            Page::Quota => "/quota",
        }
    }
}

/// One header link, as the template renders it.
pub(crate) struct NavLink {
    pub(crate) label: &'static str,
    pub(crate) path: &'static str,
    pub(crate) current: bool,
}

/// The header's links for a page that is `current` — every page in [`ALL`],
/// each marked current if and only if it is the one being rendered
/// (`T-nav-is-the-site-map`'s "the header marks exactly one link current").
pub(crate) fn links(current: Page) -> Vec<NavLink> {
    ALL.iter()
        .map(|&page| NavLink {
            label: page.label(),
            path: page.path(),
            current: page == current,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_lists_every_page() {
        let links = links(Page::Capture);
        assert_eq!(
            links.iter().map(|l| l.label).collect::<Vec<_>>(),
            vec!["Capture", "Pool", "Committed", "Quota"]
        );
    }

    #[test]
    fn links_marks_exactly_the_current_page() {
        let links = links(Page::Pool);
        let current: Vec<&str> = links
            .iter()
            .filter(|l| l.current)
            .map(|l| l.label)
            .collect();
        assert_eq!(current, vec!["Pool"]);
    }

    #[test]
    fn each_pages_own_path_is_distinct() {
        let paths: Vec<&str> = ALL.iter().map(|p| p.path()).collect();
        assert_eq!(paths, vec!["/", "/pool", "/committed", "/quota"]);
    }
}
