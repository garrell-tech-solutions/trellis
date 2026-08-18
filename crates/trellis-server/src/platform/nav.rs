//! The header every page carries: the product's three routes, and which one
//! a given page is.
//!
//! `T-package-by-business-domain`: a shared shell is not a capability, so
//! this lives in `platform` beside the composition root it complements --
//! `app::build_app` names every route once; [`ALL`] names the subset the
//! header links to, and in what order.

/// A page with a link in the header. `T-templates-take-view-models`:
/// deciding whether a page's own link is the current one is this module's
/// job, not `base.html`'s and not a handler's -- a handler says only which
/// page it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Inbox,
    LifeAreas,
    Stats,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::LifeAreas => "Life areas",
            Self::Stats => "Stats",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Inbox => "/",
            Self::LifeAreas => "/life-areas",
            Self::Stats => "/stats",
        }
    }
}

/// One link as `base.html` renders it: nothing left for the template to
/// decide, including whether it is the current page.
pub(crate) struct NavLink {
    pub(crate) label: &'static str,
    pub(crate) path: &'static str,
    pub(crate) current: bool,
}

/// Every page in the product's nav, in the header's own order -- the one
/// place that order is decided. A page added later is one more variant here
/// and one more row in this list; no page already shipped changes, and
/// `base.html` never compares a page's name to anything.
///
/// `label` and `path` are exhaustive matches, so the compiler sends the
/// author of a new variant here; this list is the one step it cannot force,
/// which is why `app`'s `every_header_link_reaches_the_page_it_names` walks
/// it rather than a list of its own.
pub(crate) const ALL: [Page; 3] = [Page::Inbox, Page::LifeAreas, Page::Stats];

/// The header's own links for a page declaring itself `current` -- one call,
/// made once per handler, so a fourth page never means editing the first
/// three.
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
    fn links_names_every_page_in_header_order() {
        let nav = links(Page::Inbox);
        assert_eq!(
            nav.iter().map(|l| l.label).collect::<Vec<_>>(),
            vec!["Inbox", "Life areas", "Stats"]
        );
    }

    #[test]
    fn each_links_path_matches_its_own_route() {
        let nav = links(Page::Inbox);
        assert_eq!(nav[0].path, "/");
        assert_eq!(nav[1].path, "/life-areas");
        assert_eq!(nav[2].path, "/stats");
    }

    #[test]
    fn exactly_the_declared_page_is_marked_current() {
        let nav = links(Page::LifeAreas);
        let current: Vec<&str> = nav.iter().filter(|l| l.current).map(|l| l.label).collect();
        assert_eq!(current, vec!["Life areas"]);
    }

    #[test]
    fn a_different_current_page_marks_a_different_link() {
        let nav = links(Page::Stats);
        let current: Vec<&str> = nav.iter().filter(|l| l.current).map(|l| l.label).collect();
        assert_eq!(current, vec!["Stats"]);
    }
}
