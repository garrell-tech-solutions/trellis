# one-screen-removed-routes-404-01: every route the demolition removed is gone, not merely unlinked
# one-screen-no-header-02: no navigation header renders while there is one screen
#
# #88 removed five routes and the six modules behind them. What is left is one
# screen -- capture and triage -- until #85 brings the Menu back.
#
# THIS FEATURE REPLACES app_shell.feature, which is deleted. That feature's
# subject was "every page carries the same navigation header", and eight of
# its ten scenarios asserted per-page shell behaviour for pages that no
# longer exist. A feature cannot shrink to the negation of its own title, so
# the surviving assertion -- that there is now no header at all -- is rehomed
# here, in a feature whose subject is true.
#
# T-nav-is-the-site-map is NOT superseded and needs no amendment. It says
# every page in the route table gets a header link; with one route the site
# map is empty, and an empty site map renders nothing. #70 wrote the
# inversion down and it applies in reverse here: if app_shell had NOT needed
# deleting, a page would still be reachable that should not be.
#
# Capture and triage are deliberately not re-asserted here. They are the
# point of the slice and the thing most at risk, and they already have
# capture_endpoint, inbox_view, triage_from_page, task_kinds,
# committed_triage_validation, quota_triage_validation,
# committed_field_domains, unknown_kind_rejection, dismiss_capture and
# context_tags between them. Restating a guarantee in a second place is how
# the two copies drift.
Feature: Trellis is one screen until the Menu returns

  Background:
    Given the trellis server is running with an empty task list

  Scenario: Every route the demolition removed is gone, not merely unlinked
    When the path "<path>" is requested
    Then the response status is "<status>"

    Examples:
      | path        | status |
      | /stats      | 404    |
      | /life-areas | 404    |
      | /free-time  | 404    |
      | /capacity   | 404    |
      | /schedule   | 404    |

  # one-screen-no-header-02: no navigation header renders while there is one screen
  Scenario: No navigation header renders while there is one screen
    When the inbox is viewed
    Then the page renders no navigation header
    And the page contains no link to another page
