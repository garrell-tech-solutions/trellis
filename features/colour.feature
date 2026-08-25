# mutation-stamp: sha256=b694a87e0c4a7eea65bc921771a7367941ef325556137c1cd8854114cae37091
# acceptance-mutation-manifest-begin
# {"version":1,"tested_at":"2026-08-24T19:20:48.128567043Z","feature_name":"Trellis follows the device's colour scheme","feature_path":"features/colour.feature","background_hash":"304f93e93e2b217b49069c091950b87589f0c7e789e2d0dc3aaa8d845450cb64","implementation_hash":"sha256:8d72a3ab6751e821b8d1fd0fb3e10830e641f11d879c7c9a4d878088249b8f6d","scenarios":[{"index":0,"name":"Every screen tells the browser it supports both colour schemes","scenario_hash":"3daf166a509ac810af9566b408123d67113a720140d43f62904c587d20e3cb7d","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-24T19:20:48.128567043Z"},{"index":1,"name":"A screen offers the browser a theme colour for each colour scheme","scenario_hash":"4a933412bb4f4bc55a6bf98541b07fe50b6a698d0ae53b836815e691101737c0","mutation_count":6,"result":{"Total":6,"Killed":6,"Survived":0,"Errors":0},"tested_at":"2026-08-24T19:20:48.128567043Z"},{"index":2,"name":"The manifest's colours are the app's own surface, no longer provisional","scenario_hash":"37cc3a8ab00c527af516b9b1c62ddc11f29d1ea9069c05ec99c8d89380f5e0ab","mutation_count":4,"result":{"Total":4,"Killed":4,"Survived":0,"Errors":0},"tested_at":"2026-08-24T19:20:48.128567043Z"}]}
# acceptance-mutation-manifest-end

# colour-both-schemes-declared-01: every screen tells the browser it supports both colour schemes
# colour-theme-colour-per-scheme-02: a screen offers the browser a theme colour for each colour scheme
# colour-manifest-colours-03: the manifest's colours are the app's own surface, no longer provisional
#
# WHAT IS ASSERTED HERE, AND WHY IT IS ONLY THE METADATA. Three scenarios,
# all of them plain HTTP facts about documents this slice changes: two meta
# tags and two manifest members. THE COLOURS THEMSELVES ARE NOT HERE, and
# that is the finding rather than the omission. Nothing in the acceptance
# runtime can observe a rendered colour: it asserts over HTTP against markup,
# and a composited background is neither. A scenario claiming "the muted text
# is legible" would be asserting something the runtime cannot check, which is
# worse than no scenario -- it would read as covered. `T-latency-is-a-qa-assertion`
# is the precedent and `qa/phone_layout.md` is the shape: a property of the
# running system, measured where the reading means something.
#
# So contrast, the dark palette, and the eleven `#fff` literals are asserted
# in `qa/colour.md` by a browser check, and the acceptance-level guarantee is
# that ALL 21 EXISTING FEATURES PASS UNTOUCHED. This changes no screen's
# content and no behaviour. If one needed editing, colour leaked into
# structure.
#
# THE THREE LITERALS THAT CANNOT BE TOKENS are why this file exists at all.
# `<meta name="color-scheme">` said `light`, and dark mode does not work while
# that is asserted -- scenario 01. `theme_color` and `background_color` are
# literal values a manifest cannot express as custom properties; #112 shipped
# them provisional and this is when they stop being -- scenario 03. And a
# manifest carries ONE of each while a browser's chrome is scheme-aware, so
# the per-scheme half lives in `<meta name="theme-color" media=...>`, which is
# the standard affordance for exactly this -- scenario 02.
#
# THE MANIFEST CARRIES THE LIGHT VALUES, AND BOTH OF THEM ARE THE APP'S OWN
# SURFACE. `background_color` paints the splash before the app has painted
# anything, and `theme_color` tints the browser and OS chrome around it; both
# answer the same question -- what does Trellis look like before you can see
# it -- and Trellis has one surface, so they are the same value. It stops
# being a brand block above a white app: a dark green strip over a near-white
# page was two-tone, and in dark mode a brand-coloured strip is a bright bar
# above a dark screen. `#fafdfe` is `--color-gray-50` in light, which is what
# `background_color` has always been; the change is that `theme_color` joins
# it and that the page adds `#091014`, `--color-gray-50` in dark, for a dark
# device.
#
# `installable.feature` IS NOT EDITED. Its scenario 05 already asserts that
# every screen's theme colour matches the manifest's, and it must keep
# passing, which pins the UNMEDIATED meta as the light one. This file asserts
# what those colours are; `installable.feature` asserts that they agree.
# Two features touching one document, split by what each is for.
#
# --- FOUR CORRECTIONS THIS SPEC MAKES TO THE BRIEF IT WAS GIVEN ------------
# Every number below was recomputed from the stylesheet's and the design
# system's own OKLCH values -- OKLCH -> OKLab -> linear sRGB -> WCAG relative
# luminance -- and reproduces the design system's table exactly where the two
# overlap. They differ where the SURFACE differs, which is the whole point.
#
# 1. GOLD IS A FOURTH FAILING TEXT COLOUR, AND NOBODY COUNTED IT. On
#    Committed a past date prints in `--color-gold`, and so does the `PAST`
#    badge beside it. In light that is 2.17:1 -- WORSE than the `gray-400`
#    2.36 this slice was filed to fix. Both issues counted only usages of the
#    three tokens they had already measured, so two rules that fail harder
#    than any of them went unlisted. THE OWNER RULED IT IN SCOPE, WITH NO
#    EXEMPTIONS: every text usage clears 4.5:1 against the surface it
#    actually sits on, and the check in `qa/colour.md` carries no exemption
#    list. How the past marker keeps its meaning is the coder's call within
#    the palette; gold is fine as a fill or a rule, and fine as text in dark
#    (7.60).
#
# 2. `gray-500` PASSES WHERE IT IS ACTUALLY USED, MOSTLY. Both issues measure
#    it at 4.39 "on the page background", `gray-100`. NO TEXT IN THIS PRODUCT
#    SITS ON `gray-100`: `html` paints it and `body` covers it, so on a phone
#    it is never visible at all. Measured against the surfaces that do carry
#    text, `gray-500` is 4.63 on the app surface (PASSES) and fails only on
#    the secondary surface -- `#tasks > li`, `.fields label` -- at 4.39, and
#    on the warning/trip panel at 4.37. Three rules, not ten. The blanket
#    swap the issue warns against would have been wrong in both directions.
#
# 3. DARK DOES NOT PASS EVERYWHERE AS AUTHORED. The design system measures
#    its dark ramp against the page and reports `gray-400` at 4.54. Against
#    the MINT PANEL it is 4.00 -- a fail, and the only one dark has. It is
#    the struck-through text of a done item inside a trip. The brief said to
#    confirm against what we ship rather than trust the table; this is what
#    that found. It disappears the moment `gray-400` stops being a text
#    colour, which is #124's fix, so it costs nothing extra -- but nothing
#    would have caught it.
#
# 4. MINT IS THE TRIP PANEL TOO, NOT ONLY THE WARNING PANEL. The design
#    system folds mint into gold's hue family reasoning that "both usages are
#    gated on hasWarning". That is true of the CANVAS. In the shipped product
#    `pool_body.html` renders every trip as `<div class="trip panel">`, and
#    `.panel` is mint -- so EVERY TRIP ON POOL GOES FROM PALE CYAN TO PALE
#    CREAM. Expected and visible, stated rather than discovered in review.
#    `T-canvas-is-authoritative-where-it-speaks` as amended: the shipped
#    implementation is the record. The rotation still stands -- the panel
#    carries a gold rule in both readings, and mint's lightness equals
#    `gray-100`'s, so rotated to blue it would vanish against the page.
#
# --- THE SURFACE STACK ----------------------------------------------------
# `trellis.css` hardcodes `#fff` eleven times and no token redefinition
# reaches a literal, so dark mode is cosmetic until they are tokenised. They
# are not all the same thing. NINE ARE BACKGROUNDS and two are text on a
# filled control.
#
# The product paints THREE surface levels and the design system names two, so
# the stack shifts one rung rather than collapsing:
#
#   app surface   `#fff`            -> `--color-gray-50`   (0.985 / 0.165)
#   secondary     `--color-gray-50` -> `--color-gray-100`  (0.967 / 0.205)
#   frame (html)  `--color-gray-100`-> `--color-gray-200`  (0.928 / 0.262)
#
# Light is preserved to within a rung: the surface drops 1.0 -> 0.985, and
# the secondary surface stays the same distance below it. THE RELATIONSHIP
# INVERTS IN DARK AND THAT IS CORRECT -- `gray-50` and `gray-100` swap order
# between the two ramps by design, so a panel that recedes by going darker in
# light recedes by going lighter in dark, which is how dark interfaces
# elevate. It is also what keeps `gray-500` legal: at `gray-50` the app
# surface gives 4.63, at `gray-100` it would give 4.39 and fail.
#
# The two `color: #fff` are text on a `primary-800` fill -- the chosen kind
# chip and the open commitment summary -- and take `--color-on-primary`,
# which exists for them. In dark `primary-800` is L 0.800, so white on it is
# 1.86:1 and the control becomes unreadable; that is the failure the check
# catches if either is missed.
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
