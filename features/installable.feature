# installable-manifest-is-linked-01: every screen links a manifest, and following the link serves one
# installable-manifest-members-02: the manifest carries the members an installable app requires
# installable-icons-resolve-03: every icon the manifest names is served, at the size it claims
# installable-maskable-icon-04: one icon is maskable, so no launcher crops a shape nobody chose
# installable-theme-colour-05: every screen declares a theme colour matching the manifest
# installable-no-service-worker-06: nothing registers a service worker
#
# WHAT IS ASSERTED HERE AND WHAT CANNOT BE. Everything above is visible over
# HTTP: the link, the document it points at, its members, and whether each
# icon it names actually resolves. NONE OF IT PROVES THE BROWSER WILL OFFER
# TO INSTALL -- that judgement is the browser's, it depends on engagement
# heuristics that vary by version, and no headless driver in this project can
# make it. THE INSTALL ITSELF IS A BY-HAND CHECK and qa/installable.md says
# so rather than implying otherwise.
#
# The same is true of the thing most likely to look wrong: AN INSTALLED
# WINDOW IS NOT A BROWSER TAB. scripts/qa/phone_layout.cjs asserts the tab
# bar sits on the viewport's bottom edge, but it drives a tab -- it cannot
# open a standalone window, so the safe-area behaviour that only appears once
# installed is unverified by anything automated. Said plainly; three
# phone-first screens already shipped on an unchecked layout.
#
# A MANIFEST IS NOT A PAGE, so T-nav-is-the-site-map does not apply to it.
# That rule is otherwise unconditional -- every page in the route table gets
# a header link -- and a reader finding a new route with no tab would be
# right to ask. It is a document the browser fetches, not a place the owner
# navigates to, and nothing links it from the tab bar.
#
# THE ICON IS PROVISIONAL AND SAYS SO. A `T` in the product's own typeface,
# white on the primary green, inset inside the maskable safe zone. The canvas
# draws no app icon -- the fourth gap it has left, after the done control,
# the date input and this -- and nothing in the repository could be reused:
# there are no images at all, only a woff2 and a stylesheet. A new palette
# and a dark mode are in flight, and a manifest's colours are LITERAL VALUES
# that cannot be CSS custom properties, so both the icon and the two hex
# values here will want redrawing. That is cheaper than guessing at a palette
# that does not exist yet.
#
# NO SERVICE WORKER, AND 06 ASSERTS ITS ABSENCE rather than leaving it
# inferred. An installed app that cannot reach the tailnet shows nothing, and
# caching the shell would wrap an app frame around an empty screen -- failing
# as a broken app rather than as a page that did not load, which is worse
# than the browser's own error. A coder reading "installable" would add one
# in good faith, which is exactly why the absence is a scenario.
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
