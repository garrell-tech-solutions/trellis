# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-21T18:37:04.074780662Z","feature_name":"Trellis serves the routes it has, and only those","feature_path":"features/one_screen.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:02c1891a283a6864d735106fede1e4b2804093516cb75878c182196d2873b1aa","scenarios":[]}
# acceptance-mutation-manifest-end

# one-screen-routes-01: a route the product has answers; every route the demolition removed is gone
#
# THE NO-HEADER SCENARIO IS GONE, not inverted. It asserted that no navigation
# renders while one screen exists, and #92 restores the tab bar with two. Its
# replacement is pool-screen-tabs-09, which asserts what the header now holds
# on both screens rather than that it is absent -- so the guarantee moved
# rather than lapsed. #70's inversion still applies: if this feature had NOT
# needed changing, the second screen shipped with no way to reach it.
#
# #88 removed five routes and the six modules behind them; #92 added /pool,
# the first Menu tab.
#
# THE `path` COLUMN IS FALSIFIABLE NOW, and it was not before. When every row
# expected 404, mutating /stats to /statX still 404d -- nine mutants survived
# this scenario on exactly that, the trap that says IF A SCENARIO'S POINT IS
# THAT NOTHING HAPPENS, ITS PARAMETERS ARE NOT UNDER TEST. #90 deferred the
# fix to the slice that would add a 200 route, and this is it: mutate / or
# /pool now and the expected status is wrong.
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
Feature: Trellis serves the routes it has, and only those

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A route the product has answers; every route the demolition removed is gone
    When the path "<path>" is requested
    Then the response status is "<status>"

    Examples:
      | path        | status |
      | /           | 200    |
      | /pool       | 200    |
      | /stats      | 404    |
      | /life-areas | 404    |
      | /capacity   | 404    |
      | /schedule   | 404    |
      | /free-time  | 404    |
