# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:26:28.358032969Z","feature_name":"Trellis serves the routes it has, and only those","feature_path":"features/one_screen.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:02c1891a283a6864d735106fede1e4b2804093516cb75878c182196d2873b1aa","scenarios":[]}
# acceptance-mutation-manifest-end

# one-screen-routes-01: a route the product has answers; every route the demolition removed is gone
#
# THE NO-HEADER SCENARIO IS GONE, not inverted. #92 restores the tab bar and
# pool-screen-tabs-09 asserts what the header now holds, so the guarantee moved
# rather than lapsed. #88 removed five routes and the six modules behind them;
# #92 added /pool and #94 /committed.
#
# THE `path` COLUMN IS FALSIFIABLE NOW, AND IT WAS NOT BEFORE. When every row
# expected 404, mutating /stats to /statX still 404d -- nine mutants survived
# this scenario on exactly that, the trap that says IF A SCENARIO'S POINT IS
# THAT NOTHING HAPPENS, ITS PARAMETERS ARE NOT UNDER TEST. #90 deferred the fix
# to the slice that would add a 200 route, and this is it.
#
# THIS FEATURE REPLACES app_shell.feature, which is deleted: eight of its ten
# scenarios asserted per-page shell behaviour for pages that no longer exist,
# and a feature cannot shrink to the negation of its own title, so the
# surviving assertion is rehomed here. T-nav-is-the-site-map is NOT superseded
# and needs no amendment -- with one route the site map is empty, and an empty
# site map renders nothing.
#
# Capture and triage are deliberately not re-asserted here. Ten other feature
# files hold them between them, and restating a guarantee in a second place is
# how the two copies drift.
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
      | /committed  | 200    |
      | /stats      | 404    |
      | /life-areas | 404    |
      | /capacity   | 404    |
      | /schedule   | 404    |
      | /free-time  | 404    |
