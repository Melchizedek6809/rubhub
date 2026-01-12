//! HTML assertion helpers using CSS selectors.
//!
//! These helpers provide more robust HTML testing that doesn't break
//! when the HTML structure changes slightly.

use scraper::{Html, Selector};

/// Parse HTML and find elements matching a CSS selector.
pub fn select(html: &str, selector: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let sel = Selector::parse(selector).expect("Invalid CSS selector");

    document
        .select(&sel)
        .map(|el| el.text().collect::<String>())
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
    let document = Html::parse_document(html);
    let sel = Selector::parse(selector).expect("Invalid CSS selector");
    assert!(
        document.select(&sel).next().is_some(),
        "Expected element '{}' to exist",
        selector
    );
}

/// Assert that no element matching the selector exists.
pub fn assert_element_not_exists(html: &str, selector: &str) {
    let document = Html::parse_document(html);
    let sel = Selector::parse(selector).expect("Invalid CSS selector");
    assert!(
        document.select(&sel).next().is_none(),
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
    let document = Html::parse_document(html);
    let sel = Selector::parse(selector).expect("Invalid CSS selector");
    document.select(&sel).count()
}
