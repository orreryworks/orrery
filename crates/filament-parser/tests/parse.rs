use filament_core::identifier::Id;
use filament_core::semantic::{Block, DiagramKind, Element, LayoutEngine, NoteAlign};
use filament_parser::{ElaborateConfig, parse};

#[test]
fn test_simple_component_diagram() {
    let source = r#"
        diagram component;
        box: Rectangle;
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;
        client -> server: "Request";
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component;
        svc as "User Service": Rectangle;
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component;
        type Button = Rectangle[fill_color="blue"];
        submit: Button;
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component;
        a: Rectangle;
        b: Rectangle;
        a -> b;
        b <- a;
        a <-> b;
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component
        missing_semicolon: Rectangle;
    "#;

    let result = parse(source, ElaborateConfig::default());
    assert!(result.is_err(), "Should fail on syntax error");

    let err = result.unwrap_err();
    assert!(!err.diagnostics().is_empty());
    let diag = &err.diagnostics()[0];
    assert!(!diag.message().is_empty());
    assert!(!diag.labels().is_empty() && !diag.labels()[0].span().is_empty());
}

#[test]
fn test_with_custom_config() {
    let source = r#"
        diagram component;
        box: Rectangle;
    "#;

    let config = ElaborateConfig::new(LayoutEngine::Sugiyama, LayoutEngine::Basic);
    let diagram = parse(source, config).expect("Failed to parse");

    assert_eq!(diagram.layout_engine(), LayoutEngine::Sugiyama);
}

#[test]
fn test_diagram_layout_attribute() {
    let source = r#"
        diagram component [layout_engine="sugiyama"];
        box: Rectangle;
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

    // Diagram-level attribute overrides config
    assert_eq!(diagram.layout_engine(), LayoutEngine::Sugiyama);
}

#[test]
fn test_with_notes() {
    let source = r#"
        diagram sequence;
        client: Rectangle;
        note [on=[client]]: "Important note";
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram sequence;
        client: Rectangle;
        note [on=[client], align="left"]: "Left note";
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        opt "Optional section" {
            client -> server: "Maybe";
        };
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        alt "Success" {
            client -> server: "OK";
        } else "Failure" {
            client -> server: "Error";
        };
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component;
        container: Rectangle {
            child1: Rectangle;
            child2: Rectangle;
        };
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram sequence;
        client: Rectangle;
        server: Rectangle;

        activate client {
            client -> server: "Request";
        };
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component;
        a: Rectangle;
        b: Rectangle;
        a -> b: "connects to";
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = r#"
        diagram component;
        parent: Rectangle {
            child: Rectangle;
        };
        external: Rectangle;
        parent::child -> external;
    "#;

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

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
    let source = "diagram component;";

    let diagram = parse(source, ElaborateConfig::default()).expect("Failed to parse");

    assert_eq!(diagram.kind(), DiagramKind::Component);
    assert!(diagram.scope().elements().is_empty());
}
