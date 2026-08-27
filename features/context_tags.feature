# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-21T04:08:05.713217523Z","feature_name":"A capture carries a context tag, and the tags autocomplete on what came before","feature_path":"features/context_tags.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:961151af8f97250f4e57762b431d0a22ef3479a560ad384b259c47e242a7bcce","scenarios":[{"index":3,"name":"Surrounding whitespace is not part of a tag","scenario_hash":"4363f9f6a696954d3be6562d588f8a551c50250719e064ed94b5bf1a5853ae73","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-21T04:08:05.713217523Z"},{"index":4,"name":"The tag control offers every tag used before, once","scenario_hash":"bf58d2ed27682d403ed96fc5d0773be9356c9dac0afb9c2bc3b9003e558d1635","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-21T04:08:05.713217523Z"},{"index":5,"name":"Two spellings differing only in case are one tag, shown as first typed","scenario_hash":"ad1ec500ca82e52144b2eedd17f3971243ca01a501ce591eea116945ff777334","mutation_count":2,"result":{"Total":2,"Killed":2,"Survived":0,"Errors":0},"tested_at":"2026-08-21T04:08:05.713217523Z"},{"index":6,"name":"A task shows the tag its capture carried","scenario_hash":"a2faa0757fbf64fc0dcbfe8fc2cee1c33c147f25e57f82bd70c28be3a5756308","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-21T04:08:05.713217523Z"}]}
# acceptance-mutation-manifest-end

# context-tags-captured-with-a-tag-01: a capture keeps the context tag it was given
# context-tags-optional-02: a capture with no context tag is accepted, not rejected
# context-tags-blank-is-absent-03: a tag of only whitespace is the same as no tag at all
# context-tags-trimmed-04: surrounding whitespace is not part of a tag
# context-tags-suggestions-05: the tag control offers every tag used before, once
# context-tags-case-is-one-tag-06: two spellings differing only in case are one tag, shown as first typed
# context-tags-survives-triage-07: a task shows the tag its capture carried
# context-tags-taggable-at-triage-08: a tag can be given at triage as well as at capture
# context-tags-escapes-hostile-text-09: a hostile tag stays escaped wherever it renders
#
# A context tag is free text and optional (D-context-tags-are-the-taxonomy): a
# typo costs one badly-grouped item, not an unschedulable one. 06 is why case
# folding matters -- a Menu showing two Home Depot lists sends the owner twice.
#
# THE ASYMMETRY WITH D-quotas-are-selected-not-typed IS DELIBERATE and worth
# stating where it will be read. A mistyped quota name splits a week's hours
# across two counters and makes both wrong; a mistyped context tag mis-files
# one item. Same product, opposite rules -- so nothing here should grow a
# managed set.
#
# 05 ASSERTS WHAT THE SERVER SENDS -- every tag used before, exactly once --
# and CANNOT assert prefix narrowing, which is the browser's and untestable
# over HTTP in this stack; qa/context_tags.md says the same rather than
# implying coverage. 06's two columns carry the same value on purpose: the
# suggestion set and the spelling each row displays are different assertions
# that happen to coincide here, and collapsing them would make the step text
# disagree with 05's for no gain.
#
# THE TAG LIVES ON THE CAPTURE and a task reads it through the capture it came
# from. One fact, one row.
#
# T-latency-is-a-qa-assertion: THE SUGGESTION SET MUST NOT BE COMPUTED IN THE
# CAPTURE PATH. It is a distinct-values query belonging to the page render, and
# a capture that pays for it has quietly moved the budget's subject.
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

  # context-tags-suggestions-05: the tag control offers every tag used before, once
  Scenario: The tag control offers every tag used before, once
    Given a capture with raw text "buy screws" tagged "@homedepot" is waiting in the untriaged queue
    And a capture with raw text "pick up milk" tagged "@supermarket" is waiting in the untriaged queue
    And a capture with raw text "return the drill" tagged "@homedepot" is waiting in the untriaged queue
    And a capture with raw text "renew the passport" is waiting in the untriaged queue
    When the inbox is viewed
    Then the context tag suggestions are exactly "<suggestions>"

    Examples:
      | suggestions              |
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

  # context-tags-survives-triage-07: a task shows the tag its capture carried
  Scenario: A task shows the tag its capture carried
    Given a capture with raw text "buy screws" tagged "@homedepot" is waiting in the untriaged queue
    When the capture is triaged as a pool task
    And the inbox is viewed
    Then the row for "buy screws" reads "Pool · <tag>"
    When the pool screen is viewed
    Then the pool screen lists "buy screws" tagged "<tag>"

    Examples:
      | tag        |
      | @homedepot |

  # context-tags-taggable-at-triage-08: a tag can be given at triage as well as at capture
  Scenario: A tag can be given at triage as well as at capture
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the capture is triaged as a pool task tagged "<tag>"
    And the inbox is viewed
    Then the pool screen lists "buy screws" tagged "<tag>"

    Examples:
      | tag        |
      | @homedepot |

  # context-tags-escapes-hostile-text-09: a hostile tag stays escaped wherever it renders
  Scenario: A hostile tag stays escaped wherever it renders
    When the quick-add box submits a capture with raw text "buy screws" tagged "<script>alert('boom')</script>"
    And the inbox is viewed
    Then the inbox does not contain an unescaped "<script>" tag
    And the inbox contains the word "boom"
