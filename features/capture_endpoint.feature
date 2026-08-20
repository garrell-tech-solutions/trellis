# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T15:02:07.939419540Z","feature_name":"Capture endpoint accepts and persists captures","feature_path":"features/capture_endpoint.feature","background_hash":"74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b","implementation_hash":"sha256:c39914c208d23416081981a3b5420870f3f8cf1f4b6ebd8bff24e7253b9ffb85","scenarios":[]}
# acceptance-mutation-manifest-end

# capture-endpoint-persists-01: a capture request persists a row and returns 201
#
# The 50 ms budget left this scenario in #88 (T-latency-is-a-qa-assertion) and
# is now qa/capture_endpoint.md's, measured against a real server on a quiet
# machine. It remains the design constraint -- capture's own module header
# cites it and T-classifier-covers-domain reasons from it -- it simply stops
# being asserted where a busy machine can falsify it.
#
# It was not merely a flake. It fired spuriously in four of six slices, and
# the harm that decided it was silent: under mutation-run contention the
# request took 1.885s, which failed cargo-mutants' UNMUTATED BASELINE, so it
# refused to test a single mutant and the whole trellis-server crate had zero
# mutation coverage with nothing connecting cause to effect.
Feature: Capture endpoint accepts and persists captures

  Scenario: Submitting a capture persists it
    Given the trellis server is running with an empty captures table
    When the QA agent sends a capture request with raw text "<raw_text>" and source "<source>"
    Then the response status is 201
    And a row exists in the captures table with raw text "<raw_text>" and source "<source>"

    Examples:
      | raw_text         | source   |
      | buy milk          | web      |
      | call the dentist  | telegram |
