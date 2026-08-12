# mutation-stamp: sha256=c2d4fb746af512b5e43f633b44f7cbf1ef36d8425610d3ac34b01da9ae5abcc5
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-12T15:20:56.915048237Z","feature_name":"Release binary is a single static executable","feature_path":"features/release_binary.feature","background_hash":"74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b","implementation_hash":"sha256:4b500984eddd5dc70d3be4f6e1cfa1b0f795eb0145bec0dcd6996f771abab157","scenarios":[]}
# acceptance-mutation-manifest-end

# release-binary-static-01: the release build produces a single static musl binary
Feature: Release binary is a single static executable

  Scenario: Building the release binary for the musl target produces a static executable
    Given the workspace is checked out
    When the release binary is built for the musl target
    Then exactly one release binary is produced
    And the binary reports no dynamic executable dependencies
