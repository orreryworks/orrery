//! Integration tests for the DiagramBuilder API
//!
//! These tests verify that the public API works and is usable.

use filament::{DiagramBuilder, config::AppConfig};

#[test]
fn test_builder_api_exists() {
    // Just verify the API compiles and can be constructed
    let source = "diagram component;";
    let _builder = DiagramBuilder::new(source);
}

#[test]
fn test_parse_simple_diagram() {
    let source = r#"
        diagram component;
        app: Rectangle;
    "#;

    let result = DiagramBuilder::new(source).parse();
    assert!(
        result.is_ok(),
        "Should parse valid diagram: {:?}",
        result.err()
    );
}

#[test]
fn test_render_simple_diagram() {
    let source = r#"
        diagram component;
        app: Rectangle [fill_color="blue"];
    "#;

    let result = DiagramBuilder::new(source).render_svg();

    if let Ok(svg) = result {
        assert!(svg.contains("<svg"), "Output should contain SVG tag");
        assert!(svg.contains("</svg>"), "Output should be complete SVG");
    } else {
        panic!("Failed to render: {:?}", result.err());
    }
}

#[test]
fn test_builder_with_config() {
    let source = "diagram component; app: Rectangle;";
    let config = AppConfig::default();

    // Just verify the API works with config
    let _result = DiagramBuilder::new(source).with_config(config).parse();

    // If it compiles and doesn't panic, the API works
}

#[test]
fn test_parse_invalid_syntax_returns_error() {
    let invalid_source = "this is not valid filament syntax!!!";

    let result = DiagramBuilder::new(invalid_source).parse();
    assert!(result.is_err(), "Should return error for invalid syntax");
}
