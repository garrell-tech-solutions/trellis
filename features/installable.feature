# mutation-stamp: sha256=be26fc83c86f58f37a36e4a9f83be4baa73f4fbfc8b5871897cf63ef164086b8
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-23T21:44:47.827585697Z","feature_name":"Trellis installs to the home screen","feature_path":"features/installable.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:d03f2fc1f482764ce739774292c0c9b7aacdef58ff7315f631e3f78c7633d3a7","scenarios":[{"index":0,"name":"Every screen links a manifest, and following the link serves one","scenario_hash":"c275b4cc996871ba4d94b31748a076a722a13d0a0cf62c0a43636c52efb3b9f0","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-23T21:44:47.827585697Z"},{"index":1,"name":"The manifest carries the members an installable app requires","scenario_hash":"83ebec378d1c616c3e961cc4ddb54968c615196b56dee48e42b8b8f394db689f","mutation_count":8,"result":{"Total":8,"Killed":8,"Survived":0,"Errors":0},"tested_at":"2026-08-23T21:44:47.827585697Z"},{"index":2,"name":"Every icon the manifest names is served, at the size it claims","scenario_hash":"d3d06b260a59516b8f2727d2fcfc67d36666c99c0f900c93ef5e60771505c597","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-23T21:44:47.827585697Z"},{"index":3,"name":"One icon is maskable, so no launcher crops a shape nobody chose","scenario_hash":"6af6ff68f6708d38d36445b796ba80222dbab266b785270af51acaffda8aa0cf","mutation_count":1,"result":{"Total":1,"Killed":1,"Survived":0,"Errors":0},"tested_at":"2026-08-23T21:44:47.827585697Z"},{"index":4,"name":"Every screen declares a theme colour matching the manifest","scenario_hash":"a21ae0ba6f690fa9c8e47c491e0597d3025f5656bb318a53e29ee3802c3f1a5f","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-23T21:44:47.827585697Z"},{"index":5,"name":"Nothing registers a service worker","scenario_hash":"6bcd9196de9beb12a36efc406d6c30b2f96158a26b10c748b5bf9b071e810b8f","mutation_count":3,"result":{"Total":3,"Killed":3,"Survived":0,"Errors":0},"tested_at":"2026-08-23T21:44:47.827585697Z"}]}
# acceptance-mutation-manifest-end

# installable-manifest-is-linked-01: every screen links a manifest, and following the link serves one
# installable-manifest-members-02: the manifest carries the members an installable app requires
# installable-icons-resolve-03: every icon the manifest names is served, at the size it claims
# installable-maskable-icon-04: one icon is maskable, so no launcher crops a shape nobody chose
# installable-theme-colour-05: every screen declares a theme colour matching the manifest
# installable-no-service-worker-06: nothing registers a service worker
#
# WHAT IS ASSERTED HERE AND WHAT CANNOT BE. Everything above is visible over
# HTTP. NONE OF IT PROVES THE BROWSER WILL OFFER TO INSTALL -- that judgement
# is the browser's and depends on engagement heuristics no headless driver here
# can make, so THE INSTALL ITSELF IS A BY-HAND CHECK in qa/installable.md. Nor
# can anything automated see the safe-area behaviour: phone_layout.cjs drives a
# tab, and AN INSTALLED WINDOW IS NOT A BROWSER TAB. Said plainly; three
# phone-first screens already shipped on an unchecked layout.
#
# A MANIFEST IS NOT A PAGE, so T-nav-is-the-site-map does not apply and nothing
# links it from the tab bar. That rule is otherwise unconditional, so a reader
# finding a new route with no tab would be right to ask.
#
# THE ICON IS PROVISIONAL AND SAYS SO: a `T` in the product's own typeface,
# white on the primary green, inset inside the maskable safe zone. The canvas
# draws no app icon and the repository has no images to reuse; a manifest's
# colours are LITERAL VALUES that cannot be custom properties, so both the icon
# and the two hex values will want redrawing once the palette lands. Cheaper
# than guessing at a palette that does not exist yet.
#
# NO SERVICE WORKER, AND 06 ASSERTS ITS ABSENCE rather than leaving it
# inferred. Caching the shell would wrap an app frame around an empty screen,
# failing as a broken app rather than as a page that did not load -- worse than
# the browser's own error. A coder reading "installable" would add one in good
# faith, which is exactly why the absence is a scenario.
Feature: Trellis installs to the home screen

  Background:
    Given the trellis server is running with an empty task list

  Scenario: Every screen links a manifest, and following the link serves one
    When the "<screen>" screen is viewed
    Then the page links a web app manifest
    And following the manifest link serves a document parsed as JSON

    Examples:
      | screen    |
      | capture   |
      | pool      |
      | committed |

  # installable-manifest-members-02: the manifest carries the members an installable app requires
  Scenario: The manifest carries the members an installable app requires
    When the manifest is fetched
    Then the manifest member "<member>" is "<value>"

    Examples:
      | member       | value      |
      | name         | Trellis    |
      | short_name   | Trellis    |
      | start_url    | /          |
      | display      | standalone |

  # installable-icons-resolve-03: every icon the manifest names is served, at the size it claims
  Scenario: Every icon the manifest names is served, at the size it claims
    When the manifest is fetched
    Then the manifest declares an icon of "<size>"
    And that icon is served as "<type>"

    Examples:
      | size    | type      |
      | 192x192 | image/png |
      | 512x512 | image/png |

  # installable-maskable-icon-04: one icon is maskable, so no launcher crops a shape nobody chose
  Scenario: One icon is maskable, so no launcher crops a shape nobody chose
    When the manifest is fetched
    Then the manifest declares an icon with purpose "<purpose>"
    And that icon is served as "image/png"

    Examples:
      | purpose  |
      | maskable |

  # installable-theme-colour-05: every screen declares a theme colour matching the manifest
  Scenario: Every screen declares a theme colour matching the manifest
    When the "<screen>" screen is viewed
    Then the page declares a theme colour
    And the page's theme colour matches the manifest's

    Examples:
      | screen    |
      | capture   |
      | pool      |
      | committed |

  # installable-no-service-worker-06: nothing registers a service worker
  Scenario: Nothing registers a service worker
    When the "<screen>" screen is viewed
    Then the page registers no service worker
    And no service worker script is served

    Examples:
      | screen    |
      | capture   |
      | pool      |
      | committed |
