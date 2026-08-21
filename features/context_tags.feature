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
# A context tag is free text and optional (D-context-tags-are-the-taxonomy):
# a typo costs one badly-grouped item, not an unschedulable one. Its whole
# value is that the second errand at a place lands in the same bucket as the
# first, which is what 06 is for -- typing slips more readily than picking,
# and a Menu showing two Home Depot lists sends the owner twice.
#
# THE ASYMMETRY IS DELIBERATE and worth stating where it will be read:
# D-quotas-are-selected-not-typed makes a quota picked and never typed,
# because a mistyped quota name splits a week's hours across two counters and
# makes both wrong. A mistyped context tag mis-files one item. Same product,
# opposite rules, for a reason -- so nothing here should grow a managed set.
#
# WHAT THE SUGGESTIONS SCENARIO CAN AND CANNOT ASSERT. The control offers the
# whole set of prior tags and the browser narrows it as the owner types. 05
# asserts what the server sends -- every tag used before, exactly once -- and
# CANNOT assert prefix narrowing, which is the browser's and untestable over
# HTTP with no browser automation in this stack. qa/context_tags.md says the
# same rather than implying coverage.
#
# THE TAG LIVES ON THE CAPTURE and a task reads it through the capture it came
# from. One fact, one row: a copy on the task is two places to edit and two
# chances to disagree. A dismissed capture keeps a tag nothing reads, which
# costs nothing -- D-kill-means-archive keeps that row regardless.
#
# 06's two columns carry the same value on purpose. The suggestion set and
# the spelling each row displays are different assertions that happen to
# coincide here, and collapsing them to one placeholder would make the step
# text disagree with 05's for no gain.
#
# T-latency-is-a-qa-assertion: this adds a field to POST /captures, which the
# 50 ms budget describes. Adding a nullable column does not change that
# endpoint's work. THE SUGGESTION SET MUST NOT BE COMPUTED IN THAT PATH --
# it is a distinct-values query belonging to the page render, and a capture
# that pays for it has quietly moved the budget's subject.
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
    Then the task list shows "buy screws" tagged "<tag>"
    And the inbox does not list "buy screws"

    Examples:
      | tag        |
      | @homedepot |

  # context-tags-taggable-at-triage-08: a tag can be given at triage as well as at capture
  Scenario: A tag can be given at triage as well as at capture
    Given a capture with raw text "buy screws" is waiting in the untriaged queue
    When the capture is triaged as a pool task tagged "<tag>"
    And the inbox is viewed
    Then the task list shows "buy screws" tagged "<tag>"

    Examples:
      | tag        |
      | @homedepot |

  # context-tags-escapes-hostile-text-09: a hostile tag stays escaped wherever it renders
  Scenario: A hostile tag stays escaped wherever it renders
    When the quick-add box submits a capture with raw text "buy screws" tagged "<script>alert('boom')</script>"
    And the inbox is viewed
    Then the inbox does not contain an unescaped "<script>" tag
    And the inbox contains the word "boom"
