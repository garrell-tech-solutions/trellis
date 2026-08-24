//! Step handlers for `features/colour.feature`: Trellis follows the
//! device's colour scheme (#124, #128).
//!
//! The Background ("the trellis server is running with an empty task list"),
//! "the "<screen>" screen is viewed", "the manifest is fetched" and "the
//! manifest member "<member>" is "<value>"" are already matched generically
//! by [`super::pool_screen::dispatch`] and [`super::installable::dispatch`],
//! tried before this module -- nothing here duplicates them. The colours
//! themselves are not asserted here at all: see this feature's own header
//! comment and `qa/colour.md` for why a rendered contrast ratio is a QA
//! concern, not an acceptance one.

use super::*;

static THEN_DECLARES_SCHEMES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the page declares the colour schemes "([^"]+)"$"#).unwrap());
static THEN_THEME_COLOUR_FOR_SCHEME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the page's theme colour for the "([^"]+)" colour scheme is "([^"]+)"$"#).unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_DECLARES_SCHEMES.captures(text) {
        return Some(dispatch_declares_schemes(world, example, &caps));
    }
    if let Some(caps) = THEN_THEME_COLOUR_FOR_SCHEME.captures(text) {
        return Some(dispatch_theme_colour_for_scheme(world, example, &caps));
    }
    None
}

/// Resolves a captured value that may be a literal or an `Examples`
/// placeholder (`<scheme>`) written down verbatim in the step text. See
/// `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// The `<meta name="color-scheme" content="...">` a page declares.
fn declared_colour_schemes(body: &str) -> Result<String, String> {
    let start_tag = r#"<meta name="color-scheme" content=""#;
    let start = body
        .find(start_tag)
        .ok_or_else(|| format!("expected a color-scheme <meta> in the page, got:\n{body}"))?;
    let after = &body[start + start_tag.len()..];
    let end = after
        .find('"')
        .ok_or_else(|| "color-scheme <meta> content has no closing quote".to_string())?;
    Ok(after[..end].to_string())
}

/// One `<meta name="theme-color" ...>` tag: its `content` and whether it
/// carries `media="(prefers-color-scheme: dark)"` -- the light one is the
/// unmediated tag (`installable-theme-colour-05` reads that one by taking
/// whichever comes first, so it must stay first), the dark one is the tag
/// this slice adds beside it.
struct ThemeColourMeta {
    content: String,
    is_dark: bool,
}

/// Every theme-color meta tag in `body`, in document order. Scans by
/// marker rather than a full HTML parse, matching this project's own
/// `T-qa-binds-tolerantly-to-markup`-adjacent convention elsewhere in this
/// harness (`trip_section`, `manifest_href`): tolerant of attribute order
/// after `content`, not of a differently-shaped tag.
fn theme_colour_metas(body: &str) -> Vec<ThemeColourMeta> {
    let marker = r#"<meta name="theme-color" content=""#;
    let mut metas = Vec::new();
    let mut offset = 0;
    while let Some(rel_start) = body[offset..].find(marker) {
        let content_start = offset + rel_start + marker.len();
        let Some(rel_quote_end) = body[content_start..].find('"') else {
            break;
        };
        let content_end = content_start + rel_quote_end;
        let Some(rel_tag_end) = body[content_end..].find('>') else {
            break;
        };
        let tag_end = content_end + rel_tag_end;
        metas.push(ThemeColourMeta {
            content: body[content_start..content_end].to_string(),
            is_dark: body[content_end..tag_end].contains("prefers-color-scheme: dark"),
        });
        offset = tag_end;
    }
    metas
}

/// The theme colour declared for `scheme` ("light" or "dark") -- light is
/// the unmediated tag, dark is the one carrying the dark media query.
fn theme_colour_for_scheme(body: &str, scheme: &str) -> Result<String, String> {
    let metas = theme_colour_metas(body);
    let wanted = match scheme {
        "dark" => true,
        "light" => false,
        other => return Err(format!("unknown colour scheme {other:?}")),
    };
    metas
        .into_iter()
        .find(|meta| meta.is_dark == wanted)
        .map(|meta| meta.content)
        .ok_or_else(|| {
            format!("expected a theme-color meta for the {scheme:?} scheme, got:\n{body}")
        })
}

fn dispatch_declares_schemes(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = super::html_body(world, "no page response recorded")?;
    let actual = declared_colour_schemes(body)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the page to declare colour schemes {expected:?}, got {actual:?}"
        ))
    }
}

fn dispatch_theme_colour_for_scheme(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let scheme = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let body = super::html_body(world, "no page response recorded")?;
    let actual = theme_colour_for_scheme(body, &scheme)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the page's {scheme:?} theme colour to be {expected:?}, got {actual:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_colour_schemes_reads_the_meta_tag() {
        let body = r#"<meta name="color-scheme" content="light dark">"#;
        assert_eq!(declared_colour_schemes(body), Ok("light dark".to_string()));
    }

    #[test]
    fn declared_colour_schemes_errors_when_absent() {
        assert!(declared_colour_schemes("<html></html>").is_err());
    }

    /// The unmediated (light) tag followed by the dark media-scoped one --
    /// the same fixture both order-sensitive tests below share.
    fn body_with_both_theme_colours() -> &'static str {
        concat!(
            r##"<meta name="theme-color" content="#f9fafb">"##,
            r##"<meta name="theme-color" content="#0b0f14" media="(prefers-color-scheme: dark)">"##
        )
    }

    #[test]
    fn theme_colour_for_scheme_reads_the_unmediated_tag_as_light() {
        assert_eq!(
            theme_colour_for_scheme(body_with_both_theme_colours(), "light"),
            Ok("#f9fafb".to_string())
        );
    }

    #[test]
    fn theme_colour_for_scheme_reads_the_dark_media_tag() {
        assert_eq!(
            theme_colour_for_scheme(body_with_both_theme_colours(), "dark"),
            Ok("#0b0f14".to_string())
        );
    }

    #[test]
    fn theme_colour_for_scheme_is_order_independent() {
        let body = concat!(
            r##"<meta name="theme-color" content="#0b0f14" media="(prefers-color-scheme: dark)">"##,
            r##"<meta name="theme-color" content="#f9fafb">"##
        );
        assert_eq!(
            theme_colour_for_scheme(body, "light"),
            Ok("#f9fafb".to_string())
        );
        assert_eq!(
            theme_colour_for_scheme(body, "dark"),
            Ok("#0b0f14".to_string())
        );
    }

    #[test]
    fn theme_colour_for_scheme_errors_when_the_scheme_is_missing() {
        let body = r##"<meta name="theme-color" content="#f9fafb">"##;
        assert!(theme_colour_for_scheme(body, "dark").is_err());
    }

    #[tokio::test]
    async fn dispatch_declares_schemes_passes_on_a_match() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<meta name="color-scheme" content="light dark">"#.to_string());
        let re = Regex::new(r#"^the page declares the colour schemes "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"the page declares the colour schemes "light dark""#)
            .unwrap();
        dispatch_declares_schemes(&mut world, &BTreeMap::new(), &caps).unwrap();
    }

    #[tokio::test]
    async fn dispatch_declares_schemes_fails_on_a_mismatch() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<meta name="color-scheme" content="light">"#.to_string());
        let re = Regex::new(r#"^the page declares the colour schemes "([^"]+)"$"#).unwrap();
        let caps = re
            .captures(r#"the page declares the colour schemes "light dark""#)
            .unwrap();
        assert!(dispatch_declares_schemes(&mut world, &BTreeMap::new(), &caps).is_err());
    }

    #[tokio::test]
    async fn dispatch_theme_colour_for_scheme_passes_on_a_match() {
        let mut world = World::new();
        world.last_html_body = Some(body_with_both_theme_colours().to_string());
        let re =
            Regex::new(r#"^the page's theme colour for the "([^"]+)" colour scheme is "([^"]+)"$"#)
                .unwrap();
        let caps = re
            .captures(r##"the page's theme colour for the "dark" colour scheme is "#0b0f14""##)
            .unwrap();
        dispatch_theme_colour_for_scheme(&mut world, &BTreeMap::new(), &caps).unwrap();
    }
}
