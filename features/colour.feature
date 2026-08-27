# mutation-stamp: sha256=808bdce62da8c5a6f6d188d86caf994d8204880fa9ae3ed859ed88b9368ea330
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-27T04:22:28.375449161Z","feature_name":"Trellis follows the device's colour scheme","feature_path":"features/colour.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:8d72a3ab6751e821b8d1fd0fb3e10830e641f11d879c7c9a4d878088249b8f6d","scenarios":[{"index":1,"name":"A screen offers the browser a theme colour for each colour scheme","scenario_hash":"c8ac42b8481232f1934845ea2a4b38a615d0fca2e7634d9cbd64c2f1640e3388","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-27T04:22:28.375449161Z"},{"index":2,"name":"The manifest's colours are the app's own surface, no longer provisional","scenario_hash":"6dc14dbfb08d9071ed0acc7bc2667f7fe72e8d31b0d7dcbf8a1421e96691a839","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-27T04:22:28.375449161Z"},{"index":0,"name":"Every screen tells the browser it supports both colour schemes","scenario_hash":"3daf166a509ac810af9566b408123d67113a720140d43f62904c587d20e3cb7d","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-24T19:20:48.128567043Z"}]}
# acceptance-mutation-manifest-end

# colour-both-schemes-declared-01: every screen tells the browser it supports both colour schemes
# colour-theme-colour-per-scheme-02: a screen offers the browser a theme colour for each colour scheme
# colour-manifest-colours-03: the manifest's colours are the app's own surface, no longer provisional
#
# THE COLOURS THEMSELVES ARE NOT ASSERTED HERE -- nothing in the acceptance
# runtime can observe a rendered colour; it asserts over HTTP against markup.
# They live in qa/colour.md's browser check (T-latency-is-a-qa-assertion), and
# the acceptance-level guarantee is that ALL 21 EXISTING FEATURES PASS
# UNTOUCHED -- if one needed editing, colour leaked into structure.
#
# WHAT IS HERE IS THE THREE LITERALS THAT CANNOT BE TOKENS.
# `<meta name="color-scheme">` said `light`, and dark mode does not work while
# that is asserted (01). A manifest carries ONE colour of each kind while
# browser chrome is scheme-aware, so the per-scheme half lives in
# `<meta name="theme-color" media=...>` (02). `theme_color` and
# `background_color` are literal values a manifest cannot express as custom
# properties; #112 shipped them provisional and this is when they stop being
# (03). Both carry `#fafdfe` -- `--color-gray-50` in light -- because both
# answer what Trellis looks like before it has painted anything, and Trellis
# has one surface. `installable.feature` IS NOT EDITED: its 05 asserts a page's
# theme colour matches the manifest's, which pins the unmediated meta as the
# light one. This file says what the colours are; that one says they agree.
#
# THE SURFACE STACK, since no token redefinition reaches a literal and the
# product paints three levels where the design system names two:
#   app surface   `#fff`            -> `--color-gray-50`   (0.985 / 0.165)
#   secondary     `--color-gray-50` -> `--color-gray-100`  (0.967 / 0.205)
#   frame (html)  `--color-gray-100`-> `--color-gray-200`  (0.928 / 0.262)
# `gray-50` and `gray-100` swap order between the ramps by design, so a panel
# that recedes by going darker in light recedes by going lighter in dark. It is
# also what keeps `gray-500` legal: 4.63 on the app surface, 4.39 on the
# secondary. The two `color: #fff` are text on a `primary-800` fill and take
# `--color-on-primary`; in dark, white on `primary-800` is 1.86:1.
#
# NO THEME TOGGLE (#118): `prefers-color-scheme` follows the device, and a
# switch is a settings surface this product does not have.

Feature: Trellis follows the device's colour scheme

  Background:
    Given the trellis server is running with an empty task list

  Scenario: Every screen tells the browser it supports both colour schemes
    When the "<screen>" screen is viewed
    Then the page declares the colour schemes "<schemes>"

    Examples:
      | screen    | schemes    |
      | capture   | light dark |
      | pool      | light dark |
      | committed | light dark |

  # colour-theme-colour-per-scheme-02: a screen offers the browser a theme colour for each colour scheme
  Scenario: A screen offers the browser a theme colour for each colour scheme
    When the "<screen>" screen is viewed
    Then the page's theme colour for the "<scheme>" colour scheme is "<colour>"

    Examples:
      | screen  | scheme | colour  |
      | capture | light  | #fafdfe |
      | capture | dark   | #091014 |

  # colour-manifest-colours-03: the manifest's colours are the app's own surface, no longer provisional
  Scenario: The manifest's colours are the app's own surface, no longer provisional
    When the manifest is fetched
    Then the manifest member "<member>" is "<value>"

    Examples:
      | member           | value   |
      | theme_color      | #fafdfe |
      | background_color | #fafdfe |
