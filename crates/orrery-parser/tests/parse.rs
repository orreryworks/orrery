//! Integration tests for the `orrery-parser` public API.
//!
//! Each test exercises the full pipeline (resolve [tokenize → parse] →
//! desugar → validate → elaborate) through the public
//! [`orrery_parser::parse`] function, using
//! [`InMemorySourceProvider`] to supply source text without touching the
//! filesystem.

use std::path::Path;

use bumpalo::Bump;

use orrery_core::{
    identifier::Id,
    semantic::{Block, Diagram, DiagramKind, Element, LayoutEngine, NoteAlign},
};
use orrery_parser::{ElaborateConfig, InMemorySourceProvider, parse};

/// Helper: parse a single source string through the full pipeline.
fn parse_source(source: &str) -> Diagram {
    let arena = Bump::new();
    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test.orr", source);
    parse(
        &arena,
        Path::new("test.orr"),
        provider,
        ElaborateConfig::default(),
    )
    .expect("parse_source: unexpected parse failure")
}

#[test]
fn test_simple_component_diagram() {
    let diagram = parse_source(
        r#"
        diagram component;
        box: Rectangle;
    "#,
    );

    assert_eq!(diagram.kind(), DiagramKind::Component);

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 1);

    match &elements[0] {
        Element::Node(node) => {
            assert_eq!(node.id(), Id::new("box"));
            assert_eq!(node.display_text(), "box");
            assert!(matches!(node.block(), Block::None));
        }
        _ => panic!("Expected Node element"),
    }
}

#[test]
fn test_simple_sequence_diagram() {
    let diagram = parse_source(
        r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;
        client -> server: "Request";
    "#,
    );

    assert_eq!(diagram.kind(), DiagramKind::Sequence);

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 3);

    // First element: client node
    match &elements[0] {
        Element::Node(node) => {
            assert_eq!(node.id(), Id::new("client"));
        }
        _ => panic!("Expected Node element for client"),
    }

    // Second element: server node
    match &elements[1] {
        Element::Node(node) => {
            assert_eq!(node.id(), Id::new("server"));
        }
        _ => panic!("Expected Node element for server"),
    }

    // Third element: relation
    match &elements[2] {
        Element::Relation(relation) => {
            assert_eq!(relation.source(), Id::new("client"));
            assert_eq!(relation.target(), Id::new("server"));
            assert_eq!(
                relation.text().map(|t| t.content().to_string()),
                Some("Request".to_string())
            );
        }
        _ => panic!("Expected Relation element"),
    }
}

#[test]
fn test_node_with_display_name() {
    let diagram = parse_source(
        r#"
        diagram component;
        svc as "User Service": Rectangle;
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 1);

    match &elements[0] {
        Element::Node(node) => {
            assert_eq!(node.id(), Id::new("svc"));
            assert_eq!(node.display_text(), "User Service");
        }
        _ => panic!("Expected Node element"),
    }
}

#[test]
fn test_with_type_definitions() {
    let diagram = parse_source(
        r#"
        diagram component;
        type Button = Rectangle[fill_color="blue"];
        submit: Button;
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 1);

    match &elements[0] {
        Element::Node(node) => {
            assert_eq!(node.id(), Id::new("submit"));
        }
        _ => panic!("Expected Node element"),
    }
}

#[test]
fn test_with_relations() {
    let diagram = parse_source(
        r#"
        diagram component;
        a: Rectangle;
        b: Rectangle;
        a -> b;
        b <- a;
        a <-> b;
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 5); // 2 nodes + 3 relations

    // Check nodes
    assert!(matches!(&elements[0], Element::Node(n) if n.id() == Id::new("a")));
    assert!(matches!(&elements[1], Element::Node(n) if n.id() == Id::new("b")));

    // Check relations
    match &elements[2] {
        Element::Relation(r) => {
            assert_eq!(r.source(), Id::new("a"));
            assert_eq!(r.target(), Id::new("b"));
        }
        _ => panic!("Expected Relation"),
    }

    match &elements[3] {
        Element::Relation(r) => {
            assert_eq!(r.source(), Id::new("b"));
            assert_eq!(r.target(), Id::new("a"));
        }
        _ => panic!("Expected Relation"),
    }

    match &elements[4] {
        Element::Relation(r) => {
            assert_eq!(r.source(), Id::new("a"));
            assert_eq!(r.target(), Id::new("b"));
        }
        _ => panic!("Expected Relation"),
    }
}

#[test]
fn test_syntax_error() {
    let arena = Bump::new();
    let mut provider = InMemorySourceProvider::new();
    provider.add_file(
        "test.orr",
        r#"
        diagram component
        missing_semicolon: Rectangle;
    "#,
    );
    let result = parse(
        &arena,
        Path::new("test.orr"),
        provider,
        ElaborateConfig::default(),
    );
    assert!(result.is_err(), "Should fail on syntax error");

    let err = result.unwrap_err();
    assert!(!err.diagnostics().is_empty());
    let diag = &err.diagnostics()[0];
    assert!(!diag.message().is_empty());
    assert!(!diag.labels().is_empty() && !diag.labels()[0].span().is_empty());
}

#[test]
fn test_with_custom_config() {
    let arena = Bump::new();
    let mut provider = InMemorySourceProvider::new();
    provider.add_file("test.orr", "diagram component;\nbox: Rectangle;");

    let config = ElaborateConfig::new(LayoutEngine::Sugiyama, LayoutEngine::Basic);
    let diagram = parse(&arena, Path::new("test.orr"), provider, config).expect("Failed to parse");

    assert_eq!(diagram.layout_engine(), LayoutEngine::Sugiyama);
}

#[test]
fn test_diagram_layout_attribute() {
    let diagram = parse_source(
        r#"
        diagram component [layout_engine="sugiyama"];
        box: Rectangle;
    "#,
    );

    // Diagram-level attribute overrides config
    assert_eq!(diagram.layout_engine(), LayoutEngine::Sugiyama);
}

#[test]
fn test_with_notes() {
    let diagram = parse_source(
        r#"
        diagram sequence;
        client: Rectangle;
        note [on=[client]]: "Important note";
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 2);

    match &elements[1] {
        Element::Note(note) => {
            assert_eq!(note.content(), "Important note");
            assert_eq!(note.on().len(), 1);
            assert_eq!(note.on()[0], Id::new("client"));
            assert_eq!(note.align(), NoteAlign::Over); // Default for sequence diagrams
        }
        _ => panic!("Expected Note element"),
    }
}

#[test]
fn test_note_with_alignment() {
    let diagram = parse_source(
        r#"
        diagram sequence;
        client: Rectangle;
        note [on=[client], align="left"]: "Left note";
    "#,
    );

    let elements = diagram.scope().elements();
    match &elements[1] {
        Element::Note(note) => {
            assert_eq!(note.align(), NoteAlign::Left);
        }
        _ => panic!("Expected Note element"),
    }
}

#[test]
fn test_with_fragments() {
    let diagram = parse_source(
        r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        opt "Optional section" {
            client -> server: "Maybe";
        };
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 3); // 2 nodes + 1 fragment

    match &elements[2] {
        Element::Fragment(fragment) => {
            assert_eq!(fragment.operation(), "opt");
            assert_eq!(fragment.sections().len(), 1);

            let section = &fragment.sections()[0];
            assert_eq!(section.title(), Some("Optional section"));
            assert_eq!(section.elements().len(), 1);

            match &section.elements()[0] {
                Element::Relation(r) => {
                    assert_eq!(r.source(), Id::new("client"));
                    assert_eq!(r.target(), Id::new("server"));
                }
                _ => panic!("Expected Relation inside fragment"),
            }
        }
        _ => panic!("Expected Fragment element"),
    }
}

#[test]
fn test_alt_fragment_with_sections() {
    let diagram = parse_source(
        r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        alt "Success" {
            client -> server: "OK";
        } else "Failure" {
            client -> server: "Error";
        };
    "#,
    );

    let elements = diagram.scope().elements();

    match &elements[2] {
        Element::Fragment(fragment) => {
            assert_eq!(fragment.operation(), "alt");
            assert_eq!(fragment.sections().len(), 2);

            assert_eq!(fragment.sections()[0].title(), Some("Success"));
            assert_eq!(fragment.sections()[1].title(), Some("Failure"));
        }
        _ => panic!("Expected Fragment element"),
    }
}

#[test]
fn test_nested_components() {
    let diagram = parse_source(
        r#"
        diagram component;
        container: Rectangle {
            child1: Rectangle;
            child2: Rectangle;
        };
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 1);

    match &elements[0] {
        Element::Node(node) => {
            assert_eq!(node.id(), Id::new("container"));

            match node.block() {
                Block::Scope(scope) => {
                    assert_eq!(scope.elements().len(), 2);

                    match &scope.elements()[0] {
                        Element::Node(child) => {
                            // Nested children have qualified IDs
                            assert_eq!(child.id(), Id::new("container::child1"));
                        }
                        _ => panic!("Expected child Node"),
                    }

                    match &scope.elements()[1] {
                        Element::Node(child) => {
                            assert_eq!(child.id(), Id::new("container::child2"));
                        }
                        _ => panic!("Expected child Node"),
                    }
                }
                _ => panic!("Expected Block::Scope"),
            }
        }
        _ => panic!("Expected Node element"),
    }
}

#[test]
fn test_activation() {
    let diagram = parse_source(
        r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        activate client {
            client -> server: "Request";
        };
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 5); // 2 nodes + activate + relation + deactivate

    match &elements[2] {
        Element::Activate(activate) => {
            assert_eq!(activate.component(), Id::new("client"));
        }
        _ => panic!("Expected Activate element"),
    }

    match &elements[3] {
        Element::Relation(r) => {
            assert_eq!(r.source(), Id::new("client"));
            assert_eq!(r.target(), Id::new("server"));
        }
        _ => panic!("Expected Relation element"),
    }

    match &elements[4] {
        Element::Deactivate(id) => {
            assert_eq!(*id, Id::new("client"));
        }
        _ => panic!("Expected Deactivate element"),
    }
}

#[test]
fn test_relation_with_label() {
    let diagram = parse_source(
        r#"
        diagram component;
        a: Rectangle;
        b: Rectangle;
        a -> b: "connects to";
    "#,
    );

    let elements = diagram.scope().elements();

    match &elements[2] {
        Element::Relation(r) => {
            let text = r.text().expect("Should have label");
            assert_eq!(text.content(), "connects to");
        }
        _ => panic!("Expected Relation"),
    }
}

#[test]
fn test_cross_level_relation() {
    let diagram = parse_source(
        r#"
        diagram component;
        parent: Rectangle {
            child: Rectangle;
        };
        external: Rectangle;
        parent::child -> external;
    "#,
    );

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 3); // parent, external, relation

    match &elements[2] {
        Element::Relation(r) => {
            // The source should be the fully qualified path
            assert_eq!(r.source(), Id::new("parent::child"));
            assert_eq!(r.target(), Id::new("external"));
        }
        _ => panic!("Expected Relation"),
    }
}

#[test]
fn test_empty_diagram() {
    let diagram = parse_source("diagram component;");

    assert_eq!(diagram.kind(), DiagramKind::Component);
    assert!(diagram.scope().elements().is_empty());
}

#[test]
fn test_import_library_types() {
    let mut provider = InMemorySourceProvider::new();
    provider.add_file(
        "shared/styles.orr",
        r#"
        library;
        type Service = Rectangle[fill_color="lightblue"];
        type Database = Oval[fill_color="lightgreen"];
    "#,
    );
    provider.add_file(
        "main.orr",
        r#"
        diagram component;
        import "shared/styles";

        api: styles::Service;
        db: styles::Database;
    "#,
    );

    let arena = Bump::new();
    let diagram = parse(
        &arena,
        Path::new("main.orr"),
        provider,
        ElaborateConfig::default(),
    )
    .expect("Failed to parse");

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 2);

    assert!(matches!(&elements[0], Element::Node(n) if n.id() == Id::new("api")));
    assert!(matches!(&elements[1], Element::Node(n) if n.id() == Id::new("db")));
}

#[test]
fn test_import_transitive_libraries() {
    let mut provider = InMemorySourceProvider::new();
    provider.add_file(
        "base.orr",
        r#"
        library;
        type Service = Rectangle[fill_color="lightblue"];
    "#,
    );
    provider.add_file(
        "extended.orr",
        r#"
        library;
        import "base";

        type SecureService = base::Service[stroke=[color="red"]];
    "#,
    );
    provider.add_file(
        "main.orr",
        r#"
        diagram component;
        import "extended";

        api: extended::SecureService;
        svc: extended::base::Service;
    "#,
    );

    let arena = Bump::new();
    let diagram = parse(
        &arena,
        Path::new("main.orr"),
        provider,
        ElaborateConfig::default(),
    )
    .expect("Failed to parse");

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 2);

    assert!(matches!(&elements[0], Element::Node(n) if n.id() == Id::new("api")));
    assert!(matches!(&elements[1], Element::Node(n) if n.id() == Id::new("svc")));
}

#[test]
fn test_import_diamond_dependency() {
    let mut provider = InMemorySourceProvider::new();
    provider.add_file(
        "base.orr",
        r#"
        library;
        type Service = Rectangle[fill_color="lightblue"];
    "#,
    );
    provider.add_file(
        "ext_a.orr",
        r#"
        library;
        import "base";

        type ServiceA = base::Service[stroke=[color="red"]];
    "#,
    );
    provider.add_file(
        "ext_b.orr",
        r#"
        library;
        import "base";

        type ServiceB = base::Service[stroke=[color="blue"]];
    "#,
    );
    provider.add_file(
        "main.orr",
        r#"
        diagram component;
        import "ext_a";
        import "ext_b";

        a: ext_a::ServiceA;
        b: ext_b::ServiceB;
    "#,
    );

    let arena = Bump::new();
    let diagram = parse(
        &arena,
        Path::new("main.orr"),
        provider,
        ElaborateConfig::default(),
    )
    .expect("Failed to parse");

    let elements = diagram.scope().elements();
    assert_eq!(elements.len(), 2);
}

#[test]
fn test_error_file_not_found() {
    let mut provider = InMemorySourceProvider::new();
    provider.add_file(
        "main.orr",
        r#"
        diagram component;
        import "nonexistent";
        box: Rectangle;
    "#,
    );

    let arena = Bump::new();
    let result = parse(
        &arena,
        Path::new("main.orr"),
        provider,
        ElaborateConfig::default(),
    );
    assert!(result.is_err(), "Should fail on missing import");

    let err = result.unwrap_err();
    let diag = &err.diagnostics()[0];
    // E400 — file not found
    assert!(
        diag.message().contains("cannot find file"),
        "Expected file-not-found error, got: {}",
        diag.message()
    );
}

#[test]
fn test_error_circular_dependency() {
    let mut provider = InMemorySourceProvider::new();
    provider.add_file(
        "a.orr",
        r#"
        library;
        import "b";
    "#,
    );
    provider.add_file(
        "b.orr",
        r#"
        library;
        import "a";
    "#,
    );

    let arena = Bump::new();
    let result = parse(
        &arena,
        Path::new("a.orr"),
        provider,
        ElaborateConfig::default(),
    );
    assert!(result.is_err(), "Should fail on circular dependency");

    let err = result.unwrap_err();
    let diag = &err.diagnostics()[0];
    // E401 — circular dependency
    assert!(
        diag.message().contains("circular dependency"),
        "Expected circular dependency error, got: {}",
        diag.message()
    );
}

#[test]
fn test_error_missing_root_file() {
    let provider = InMemorySourceProvider::new();

    let arena = Bump::new();
    let result = parse(
        &arena,
        Path::new("does_not_exist.orr"),
        provider,
        ElaborateConfig::default(),
    );
    assert!(result.is_err(), "Should fail on missing root file");

    let err = result.unwrap_err();
    let diag = &err.diagnostics()[0];
    assert!(
        diag.message().contains("cannot find file"),
        "Expected file-not-found error, got: {}",
        diag.message()
    );
}
