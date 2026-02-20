//! Integration tests for the DiagramBuilder API
//!
//! These tests verify that the public API works and is usable.

use orrery::{DiagramBuilder, config::AppConfig};

#[test]
fn test_builder_api_exists() {
    // Just verify the API compiles and can be constructed
    let _builder = DiagramBuilder::default();
}

#[test]
fn test_parse_simple_diagram() {
    let source = r#"
        diagram component;
        app: Rectangle;
    "#;

    let builder = DiagramBuilder::default();
    let result = builder.parse(source);
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

    let builder = DiagramBuilder::default();
    let diagram = builder.parse(source).expect("Failed to parse diagram");
    let result = builder.render_svg(&diagram);

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
    let builder = DiagramBuilder::new(config);
    let _result = builder.parse(source);

    // If it compiles and doesn't panic, the API works
}

#[test]
fn test_parse_invalid_syntax_returns_error() {
    let invalid_source = "this is not valid orrery syntax!!!";

    let builder = DiagramBuilder::default();
    let result = builder.parse(invalid_source);
    assert!(result.is_err(), "Should return error for invalid syntax");
}

#[test]
fn test_builder_reusability() {
    let source1 = "diagram component; app1: Rectangle;";
    let source2 = "diagram component; app2: Oval;";

    let builder = DiagramBuilder::default();

    // Parse and render first diagram
    let diagram1 = builder.parse(source1).expect("Failed to parse diagram1");
    let svg1 = builder
        .render_svg(&diagram1)
        .expect("Failed to render diagram1");

    // Reuse same builder for second diagram
    let diagram2 = builder.parse(source2).expect("Failed to parse diagram2");
    let svg2 = builder
        .render_svg(&diagram2)
        .expect("Failed to render diagram2");

    assert!(svg1.contains("<svg"), "First SVG should be valid");
    assert!(svg2.contains("<svg"), "Second SVG should be valid");
}
