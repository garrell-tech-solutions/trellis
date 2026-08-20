# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-20T21:20:21.461565878Z","feature_name":"A capture carries a context tag, and the tags autocomplete on what came before","feature_path":"features/context_tags.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:961151af8f97250f4e57762b431d0a22ef3479a560ad384b259c47e242a7bcce","scenarios":[{"index":3,"name":"Surrounding whitespace is not part of a tag","scenario_hash":"4363f9f6a696954d3be6562d588f8a551c50250719e064ed94b5bf1a5853ae73","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-20T21:20:21.461565878Z"},{"index":4,"name":"The tag control offers every tag used before","scenario_hash":"3da9687fb520f8152c22d560f9ef0554c7d06ee4974ed11e5a650ce584587aaa","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-20T21:20:21.461565878Z"},{"index":5,"name":"Two spellings differing only in case are one tag, shown as first typed","scenario_hash":"ad1ec500ca82e52144b2eedd17f3971243ca01a501ce591eea116945ff777334","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-20T21:20:21.461565878Z"}]}
# acceptance-mutation-manifest-end

# context-tags-captured-with-a-tag-01: a capture keeps the context tag it was given
# context-tags-optional-02: a capture with no context tag is accepted, not rejected
# context-tags-blank-is-absent-03: a tag of only whitespace is the same as no tag at all
# context-tags-trimmed-04: surrounding whitespace is not part of a tag
# context-tags-suggestions-05: the tag control offers every tag used before
# context-tags-case-is-one-tag-06: two spellings differing only in case are one tag, shown as first typed
# context-tags-taggable-at-triage-07: a tag can be given at triage as well as at capture
# context-tags-escapes-hostile-text-08: a hostile tag stays escaped wherever it renders
#
# A context tag is free text and optional (D-context-tags-are-the-taxonomy):
# a typo costs one badly-grouped item, not an unschedulable one. Its whole
# value is that the second errand at a place lands in the same bucket as the
# first, which is why 06 exists -- typing is likelier to slip than picking,
# and a Menu showing two Home Depot lists sends the owner twice.
#
# WHAT THE SUGGESTIONS SCENARIO CAN AND CANNOT ASSERT. The control offers the
# whole set of prior tags and the browser narrows it as the owner types. So
# 05 asserts what the server sends -- every tag used before, exactly once --
# and CANNOT assert prefix filtering, which is the browser's and untestable
# over HTTP with no browser automation in this stack. qa/context_tags.md
# says the same rather than implying coverage.
#
# The tag lives on the CAPTURE and a task reads it through the capture it
# came from. One fact, one row: a tag copied to the task at triage would be
# two places to edit and two chances to disagree, which is T-archived-at-only's
# shape. A dismissed capture keeps a tag nothing reads, which costs nothing --
# D-kill-means-archive keeps that row regardless.
Feature: A capture carries a context tag, and the tags autocomplete on what came before

  Background:
    Given the trellis server is running with an empty task list

  Scenario: A capture keeps the context tag it was given
    When the quick-add box submits a capture with raw text "buy screws" tagged "<tag>"
    Then the quick-add submission is accepted
    And the inbox lists "buy screws" tagged "<tag>"

    Examples:
      | tag        |
      | @homedepot |

  # context-tags-optional-02: a capture with no context tag is accepted, not rejected
  Scenario: A capture with no context tag is accepted, not rejected
    When the quick-add box submits a capture with raw text "buy screws" with no context tag
    Then the quick-add submission is accepted
    And the inbox lists "buy screws"
    And the inbox shows no context tag for "buy screws"

  # context-tags-blank-is-absent-03: a tag of only whitespace is the same as no tag at all
  Scenario: A tag of only whitespace is the same as no tag at all
    When the quick-add box submits a capture with raw text "buy screws" tagged "   "
    Then the quick-add submission is accepted
    And the inbox shows no context tag for "buy screws"

  # context-tags-trimmed-04: surrounding whitespace is not part of a tag
  Scenario: Surrounding whitespace is not part of a tag
    When the quick-add box submits a capture with raw text "pick up milk" tagged "  @supermarket  "
    Then the inbox lists "pick up milk" tagged "<tag>"

    Examples:
      | tag          |
      | @supermarket |

  # context-tags-suggestions-05: the tag control offers every tag used before
  Scenario: The tag control offers every tag used before
    Given a capture with raw text "buy screws" tagged "@homedepot" is waiting in the untriaged queue
    And a capture with raw text "pick up milk" tagged "@supermarket" is waiting in the untriaged queue
    And a capture with raw text "return the drill" tagged "@homedepot" is waiting in the untriaged queue
    When the inbox is viewed
    Then the context tag suggestions are exactly "<suggestions>"

    Examples:
      | suggestions             |
      | @homedepot, @supermarket |

  # context-tags-case-is-one-tag-06: two spellings differing only in case are one tag, shown as first typed
  Scenario: Two spellings differing only in case are one tag, shown as first typed
    Given a capture with raw text "buy screws" tagged "@HomeDepot" is waiting in the untriaged queue
    When the quick-add box submits a capture with raw text "return the drill" tagged "@homedepot"
    And the inbox is viewed
    Then the context tag suggestions are exactly "<suggestions>"
    And the inbox lists "return the drill" tagged "<tag>"
    And the inbox lists "buy screws" tagged "<tag>"

    Examples:
      | suggestions | tag        |
      | @HomeDepot  | @HomeDepot |

  # context-tags-taggable-at-triage-07: a tag can be given at triage as well as at capture
  Scenario: A tag can be given at triage as well as at capture
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the capture is triaged as a pool task tagged "<tag>"
    And the inbox is viewed
    Then the task list shows "buy screws" tagged "<tag>"

    Examples:
      | tag        |
      | @homedepot |

  # context-tags-escapes-hostile-text-08: a hostile tag stays escaped wherever it renders
  Scenario: A hostile tag stays escaped wherever it renders
    When the quick-add box submits a capture with raw text "buy screws" tagged "<script>alert('boom')</script>"
    And the inbox is viewed
    Then the inbox does not contain an unescaped "<script>" tag
    And the inbox contains the word "boom"
