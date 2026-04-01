//! Integration tests for the DiagramBuilder API
//!
//! These tests verify that the public API works and is usable.

use std::path::Path;

use orrery::{DiagramBuilder, InMemorySourceProvider, config::AppConfig};

#[test]
fn test_builder_api_exists() {
    // Just verify the API compiles and can be constructed
    let provider = InMemorySourceProvider::new();
    let _builder = DiagramBuilder::new(AppConfig::default(), &provider);
}

#[test]
fn test_parse_simple_diagram() {
    let source = r#"
        diagram component;
        app: Rectangle;
    "#;

    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test.orr", source);

    let builder = DiagramBuilder::new(AppConfig::default(), &provider);
    let result = builder.parse(Path::new("test.orr"));
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

    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test.orr", source);

    let builder = DiagramBuilder::new(AppConfig::default(), &provider);
    let diagram = builder
        .parse(Path::new("test.orr"))
        .expect("Failed to parse diagram");
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

    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test.orr", source);

    // Just verify the API works with config
    let builder = DiagramBuilder::new(config, &provider);
    let _result = builder.parse(Path::new("test.orr"));

    // If it compiles and doesn't panic, the API works
}

#[test]
fn test_parse_invalid_syntax_returns_error() {
    let invalid_source = "this is not valid orrery syntax!!!";

    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test.orr", invalid_source);

    let builder = DiagramBuilder::new(AppConfig::default(), &provider);
    let result = builder.parse(Path::new("test.orr"));
    assert!(result.is_err(), "Should return error for invalid syntax");
}

#[test]
fn test_builder_reusability() {
    let source1 = "diagram component; app1: Rectangle;";
    let source2 = "diagram component; app2: Oval;";

    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test1.orr", source1);
    provider.add_file("test2.orr", source2);

    let builder = DiagramBuilder::new(AppConfig::default(), &provider);

    // Parse and render first diagram
    let diagram1 = builder
        .parse(Path::new("test1.orr"))
        .expect("Failed to parse diagram1");
    let svg1 = builder
        .render_svg(&diagram1)
        .expect("Failed to render diagram1");

    // Reuse same builder for second diagram
    let diagram2 = builder
        .parse(Path::new("test2.orr"))
        .expect("Failed to parse diagram2");
    let svg2 = builder
        .render_svg(&diagram2)
        .expect("Failed to render diagram2");

    assert!(svg1.contains("<svg"), "First SVG should be valid");
    assert!(svg2.contains("<svg"), "Second SVG should be valid");
}
