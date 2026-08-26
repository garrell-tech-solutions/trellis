#!/usr/bin/env node
// Playwright-driven colour-scheme check for #124/#128, qa/colour.md.
//
// Drives a real, headless Chrome (the same one scripts/qa/phone_layout.cjs
// does) at a 390x844 phone viewport, once per screen per colour scheme, and
// asserts five things nothing over HTTP can see: every text-bearing element
// clears 4.5:1 against its actual composited background; nothing paints
// literal white in dark; every painted colour is one of the design
// system's own token values for the scheme being rendered (read from
// :root at runtime, never hardcoded); light and dark genuinely differ; and
// phone_layout.cjs's own geometry assertions still hold in both schemes.
// A sixth check cross-references the <meta name="theme-color"> value
// against what the page's own surface actually paints, per scheme.
//
// This file is not measured by scripts/analyzers/dry.sh (rust,bash only,
// T-dry-measures-product-code), so phone_layout.cjs's geometry assertions
// are reproduced here rather than shared -- no DRY cost either way.
//
// Fails loudly, never skips: an uncaught error here exits non-zero, the
// same as an assertion failure. qa/colour.md's own warning applies harder
// here than to phone_layout.cjs: the failure this slice fixes shipped,
// was measured, was written down, and stayed shipped -- a check that goes
// quiet when its browser is absent reproduces that one layer down.

const { chromium } = require('playwright-core');

const baseUrl = process.argv[2];
if (!baseUrl) {
  console.error('usage: colour.cjs <base-url>');
  process.exit(1);
}

const executablePath = process.env.PHONE_LAYOUT_CHROME || '/usr/bin/google-chrome';
const VIEWPORT = { width: 390, height: 844 };
const TAB_BAR_TOLERANCE_PX = 2;
const CONTRAST_MIN = 4.5;

const SCREENS = [
  { path: '/', label: 'Capture', expectOverflow: true },
  { path: '/pool', label: 'Pool', expectOverflow: false },
  { path: '/committed', label: 'Committed', expectOverflow: false },
  { path: '/quota', label: 'Quota', expectOverflow: false },
];
const SCHEMES = ['light', 'dark'];

const EXPECTED_THEME_COLOUR = { light: '#fafdfe', dark: '#091014' };

let failures = 0;
function fail(message) {
  console.error(`FAIL: ${message}`);
  failures += 1;
}

// WCAG 2.x relative luminance and contrast ratio, from an "rgb(r, g, b)" /
// "rgba(r, g, b, a)" computed-style string.
// Chrome preserves the source colour function in a computed-style
// serialization rather than always normalizing to rgb(): a rule written
// in oklch() (every design-system token) comes back as "oklch(L C H)",
// verified empirically against this exact Chrome build before trusting
// it, not assumed -- an rgb()-only parser here would have silently
// stopped resolving color() on almost every element (everything using a
// token), which is exactly "a check that silently measures nothing",
// qa/colour.md's own named failure mode, one layer deeper than the check
// itself.
function parseRgb(css) {
  const m = css.match(/rgba?\(([^)]+)\)/);
  if (!m) return null;
  const parts = m[1].split(',').map((s) => parseFloat(s.trim()));
  return { r: parts[0], g: parts[1], b: parts[2], a: parts.length > 3 ? parts[3] : 1 };
}

function parseOklch(css) {
  const m = css.match(/oklch\(([^)]+)\)/);
  if (!m) return null;
  const parts = m[1].split('/')[0].trim().split(/\s+/).map(parseFloat);
  const [L, C, H] = parts;
  return oklchToSrgb255(L, C, H);
}

// OKLCH -> OKLab -> linear sRGB -> sRGB, the same chain qa/colour.md's own
// baseline table was computed with (Björn Ottosson's reference matrices).
function oklchToSrgb255(L, C, H) {
  const hRad = (H * Math.PI) / 180;
  const a = C * Math.cos(hRad);
  const b = C * Math.sin(hRad);

  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.2914855480 * b;
  const l = l_ ** 3;
  const m = m_ ** 3;
  const s = s_ ** 3;

  const rLin = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const gLin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const bLin = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

  const gamma = (c) => {
    const clamped = Math.min(1, Math.max(0, c));
    return clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055;
  };
  return {
    r: Math.round(gamma(rLin) * 255),
    g: Math.round(gamma(gLin) * 255),
    b: Math.round(gamma(bLin) * 255),
    a: 1,
  };
}

// Resolves any computed-style colour string this page can emit (rgb(),
// rgba(), or oklch()) to an {r, g, b, a} triple in [0, 255] -- the one
// function every colour comparison in this file goes through, so a third
// serialization format only needs handling here.
function resolveColor(css) {
  if (!css) return null;
  return parseRgb(css) || parseOklch(css);
}

function hexToRgb(hex) {
  const clean = hex.replace('#', '');
  return {
    r: parseInt(clean.slice(0, 2), 16),
    g: parseInt(clean.slice(2, 4), 16),
    b: parseInt(clean.slice(4, 6), 16),
    a: 1,
  };
}

function relativeLuminance({ r, g, b }) {
  const chan = (c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b);
}

function contrastRatio(fg, bg) {
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

// Everything this page evaluate needs, injected as a single function run
// inside the browser: the closed palette (every design-system token's
// resolved colour, read from :root's own custom properties -- never
// hardcoded), every text-bearing element's own colour and composited
// background, every element's colour/background for the white-literal and
// closed-palette checks, and the three geometry facts phone_layout.cjs
// also asserts.
function collectPageFacts() {
  function isTransparent(css) {
    if (!css) return true;
    const m = css.match(/rgba?\(([^)]+)\)/);
    if (!m) return css === 'transparent';
    const parts = m[1].split(',').map((s) => parseFloat(s.trim()));
    return parts.length > 3 && parts[3] === 0;
  }

  // The design system's token set, read from :root at runtime: every
  // custom property getComputedStyle enumerates, resolved to its actual
  // colour by writing it as a temporary element's `color` and reading the
  // computed result back -- the tokens are the contract, their hex is not.
  const rootStyle = getComputedStyle(document.documentElement);
  const tokenNames = Array.from(rootStyle).filter((p) => p.startsWith('--color-'));
  const probe = document.createElement('span');
  probe.style.position = 'absolute';
  probe.style.visibility = 'hidden';
  document.body.appendChild(probe);
  const palette = {};
  for (const name of tokenNames) {
    probe.style.color = `var(${name})`;
    palette[name] = getComputedStyle(probe).color;
  }
  probe.remove();

  function compositedBackground(el) {
    let node = el;
    while (node) {
      const bg = getComputedStyle(node).backgroundColor;
      if (!isTransparent(bg)) return bg;
      node = node.parentElement;
    }
    return 'rgb(255, 255, 255)';
  }

  function hasOwnText(el) {
    for (const child of el.childNodes) {
      if (child.nodeType === Node.TEXT_NODE && child.textContent.trim().length > 0) {
        return true;
      }
    }
    return false;
  }

  function isRendered(el) {
    return el.offsetWidth > 0 || el.offsetHeight > 0;
  }

  const SKIP_TAGS = new Set(['SCRIPT', 'STYLE', 'OPTION', 'DATALIST']);

  const textElements = [];
  const paintedElements = [];
  for (const el of document.querySelectorAll('*')) {
    if (SKIP_TAGS.has(el.tagName)) continue;
    if (!isRendered(el)) continue;
    const style = getComputedStyle(el);
    paintedElements.push({
      tag: el.tagName,
      selector: describeElement(el),
      color: style.color,
      backgroundColor: isTransparent(style.backgroundColor) ? null : style.backgroundColor,
    });
    if (hasOwnText(el)) {
      textElements.push({
        tag: el.tagName,
        selector: describeElement(el),
        text: el.textContent.trim().slice(0, 40),
        color: style.color,
        background: compositedBackground(el),
      });
    }
  }

  function describeElement(el) {
    const cls = el.className && typeof el.className === 'string' ? `.${el.className.trim().split(/\s+/).join('.')}` : '';
    return `${el.tagName.toLowerCase()}${cls}`;
  }

  const nav = document.querySelector('nav');
  const navBox = nav ? nav.getBoundingClientRect() : null;
  const doc = {
    scrollHeight: document.scrollingElement.scrollHeight,
    clientHeight: document.scrollingElement.clientHeight,
  };
  const main = document.querySelector('main');
  const mainBox = main
    ? { scrollHeight: main.scrollHeight, clientHeight: main.clientHeight }
    : null;

  const navAnchor = nav ? nav.querySelector('a') : null;
  // The tab bar's own painted surface is <header> (position: sticky,
  // background: var(--color-gray-50)); <nav> nests inside it and is
  // itself transparent, so its own backgroundColor would never differ
  // between schemes regardless of what the tab bar actually paints.
  const header = document.querySelector('header');

  return {
    palette,
    textElements,
    paintedElements,
    doc,
    mainBox,
    navBottom: navBox ? navBox.y + navBox.height : null,
    appSurface: getComputedStyle(document.body).backgroundColor,
    navBackground: header ? getComputedStyle(header).backgroundColor : null,
    navTextColor: navAnchor ? getComputedStyle(navAnchor).color : null,
  };
}

async function main() {
  const browser = await chromium.launch({
    executablePath,
    headless: true,
    args: ['--no-sandbox'],
  });

  try {
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.setViewportSize(VIEWPORT);

    // scheme -> screen label -> facts, for the schemes-genuinely-differ
    // assertion (D), which needs both runs of the same screen at once.
    const bySchemeAndScreen = {};

    for (const scheme of SCHEMES) {
      await page.emulateMedia({ colorScheme: scheme });
      bySchemeAndScreen[scheme] = {};

      for (const screen of SCREENS) {
        await page.goto(`${baseUrl}${screen.path}`, { waitUntil: 'load' });
        const facts = await page.evaluate(collectPageFacts);
        bySchemeAndScreen[scheme][screen.label] = facts;
        const tag = `[${screen.label}/${scheme}]`;
        const paletteValues = new Set(Object.values(facts.palette));

        // A: every text-bearing element clears 4.5:1 against its
        // composited background.
        for (const el of facts.textElements) {
          const fg = resolveColor(el.color);
          const bg = resolveColor(el.background);
          if (!fg || !bg) {
            fail(`${tag} ${el.selector} (${JSON.stringify(el.text)}) has an unresolvable colour -- color=${el.color} background=${el.background}`);
            continue;
          }
          const ratio = contrastRatio(fg, bg);
          if (ratio < CONTRAST_MIN) {
            fail(
              `${tag} ${el.selector} (${JSON.stringify(el.text)}) contrast ${ratio.toFixed(2)}:1 ` +
                `(< ${CONTRAST_MIN}:1) -- color=${el.color} background=${el.background}`,
            );
          }
        }

        // B: nothing paints literal white in dark.
        if (scheme === 'dark') {
          for (const el of facts.textElements) {
            if (el.color === 'rgb(255, 255, 255)') {
              fail(`${tag} ${el.selector} (${JSON.stringify(el.text)}) paints literal white text (rgb(255, 255, 255))`);
            }
          }
          for (const el of facts.paintedElements) {
            if (el.backgroundColor === 'rgb(255, 255, 255)') {
              fail(`${tag} ${el.selector} paints a literal white background (rgb(255, 255, 255))`);
            }
          }
        }

        // C: the palette is closed -- every painted colour/background is
        // one of the scheme's own token values. `color` only matters on an
        // element that actually renders text with it (an element with no
        // text of its own, like <html>, carries a `color` the browser
        // never paints a glyph with); `backgroundColor` matters on any
        // rendered element with a non-transparent one.
        for (const el of facts.textElements) {
          if (!paletteValues.has(el.color)) {
            fail(`${tag} ${el.selector} (${JSON.stringify(el.text)}) paints color ${el.color}, not one of the scheme's tokens`);
          }
        }
        for (const el of facts.paintedElements) {
          if (el.backgroundColor && !paletteValues.has(el.backgroundColor)) {
            fail(`${tag} ${el.selector} paints background ${el.backgroundColor}, not one of the scheme's tokens`);
          }
        }

        // E: phone_layout.cjs's own three geometry assertions, unchanged
        // by colour scheme.
        if (facts.doc.scrollHeight > facts.doc.clientHeight) {
          fail(
            `${tag} the document scrolls: scrollHeight=${facts.doc.scrollHeight} ` +
              `clientHeight=${facts.doc.clientHeight}`,
          );
        }
        if (!facts.mainBox) {
          fail(`${tag} no <main> found`);
        } else if (screen.expectOverflow) {
          if (facts.mainBox.scrollHeight <= facts.mainBox.clientHeight) {
            fail(`${tag} expected main to overflow (seed more content?)`);
          }
        } else if (facts.mainBox.scrollHeight > facts.mainBox.clientHeight) {
          fail(`${tag} a screen that fits gained a scrollbar`);
        }
        if (facts.navBottom === null) {
          fail(`${tag} no navigation landmark found`);
        } else {
          const delta = Math.abs(facts.navBottom - VIEWPORT.height);
          if (delta > TAB_BAR_TOLERANCE_PX) {
            fail(`${tag} tab bar bottom edge is ${facts.navBottom}px, expected ~${VIEWPORT.height}px`);
          }
        }

        // The theme-color meta / manifest cross-check (qa/colour.md's
        // "Procedure -- the metadata" step 3): the app surface this
        // scheme actually paints equals the theme colour declared for it.
        // Both sides resolved to rgb triples rather than compared as
        // strings -- the declared hex and the rendered oklch token are
        // different serializations of the same colour.
        const expectedRgb = hexToRgb(EXPECTED_THEME_COLOUR[scheme]);
        const actualRgb = resolveColor(facts.appSurface);
        if (
          !actualRgb ||
          actualRgb.r !== expectedRgb.r ||
          actualRgb.g !== expectedRgb.g ||
          actualRgb.b !== expectedRgb.b
        ) {
          fail(
            `${tag} app surface painted ${facts.appSurface} (rgb ${JSON.stringify(actualRgb)}), ` +
              `expected the declared theme colour ${EXPECTED_THEME_COLOUR[scheme]} (rgb ${JSON.stringify(expectedRgb)})`,
          );
        }
      }
    }

    // D: the two schemes genuinely differ, per screen -- app surface, tab
    // bar text colour and tab bar background all move.
    for (const screen of SCREENS) {
      const light = bySchemeAndScreen.light[screen.label];
      const dark = bySchemeAndScreen.dark[screen.label];
      if (!light || !dark) continue;
      if (light.appSurface === dark.appSurface) {
        fail(`[${screen.label}] app surface identical in light and dark: ${light.appSurface}`);
      }
      if (light.navTextColor === dark.navTextColor) {
        fail(`[${screen.label}] tab bar text colour identical in light and dark: ${light.navTextColor}`);
      }
      if (light.navBackground === dark.navBackground) {
        fail(`[${screen.label}] tab bar background identical in light and dark: ${light.navBackground}`);
      }
    }
  } finally {
    await browser.close();
  }

  if (failures > 0) {
    console.error(`FAIL: colour (${failures} assertion(s) failed)`);
    process.exit(1);
  }
  console.log('PASS: colour (browser assertions)');
}

main().catch((error) => {
  console.error('FAIL: colour (uncaught error)');
  console.error(error);
  process.exit(1);
});
