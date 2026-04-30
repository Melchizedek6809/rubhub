//! HTML assertion helpers using CSS selectors.
//!
//! These helpers provide more robust HTML testing that doesn't break
//! when the HTML structure changes slightly.

/// Parse HTML and find elements matching a CSS selector.
pub fn select(html: &str, selector: &str) -> Vec<String> {
    let document = parse(html);
    let parser = document.parser();

    document
        .query_selector(selector)
        .expect("Invalid CSS selector")
        .filter_map(|handle| handle.get(parser))
        .map(|node| node.inner_text(parser).into_owned())
        .collect()
}

/// Check if any element matching the selector contains the expected text.
pub fn has_element_with_text(html: &str, selector: &str, expected: &str) -> bool {
    select(html, selector)
        .iter()
        .any(|text| text.contains(expected))
}

/// Assert that an element matching the selector contains the expected text.
pub fn assert_element_contains(html: &str, selector: &str, expected: &str) {
    let texts = select(html, selector);
    assert!(
        texts.iter().any(|text| text.contains(expected)),
        "Expected element '{}' to contain '{}', but found: {:?}",
        selector,
        expected,
        texts
    );
}

/// Assert that an element matching the selector exists.
pub fn assert_element_exists(html: &str, selector: &str) {
    let document = parse(html);
    assert!(
        document
            .query_selector(selector)
            .expect("Invalid CSS selector")
            .next()
            .is_some(),
        "Expected element '{}' to exist",
        selector
    );
}

/// Assert that no element matching the selector exists.
pub fn assert_element_not_exists(html: &str, selector: &str) {
    let document = parse(html);
    assert!(
        document
            .query_selector(selector)
            .expect("Invalid CSS selector")
            .next()
            .is_none(),
        "Expected element '{}' to NOT exist",
        selector
    );
}

/// Get the text content of the first element matching the selector.
pub fn get_text(html: &str, selector: &str) -> Option<String> {
    select(html, selector).into_iter().next()
}

/// Count elements matching a selector.
pub fn count_elements(html: &str, selector: &str) -> usize {
    parse(html)
        .query_selector(selector)
        .expect("Invalid CSS selector")
        .count()
}

fn parse(html: &str) -> tl::VDom<'_> {
    tl::parse(html, tl::ParserOptions::default()).expect("Invalid HTML")
}
