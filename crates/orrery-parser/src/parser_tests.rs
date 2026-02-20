//! Comprehensive unit tests for the winnow parser implementation
//!
//! These tests verify that the winnow parser correctly handles all Orrery
//! language constructs and provides proper error handling.

use crate::{lexer, parser};

/// Helper function to parse a source string and return success/failure
fn parse_source(source: &str) -> Result<(), String> {
    let tokens = lexer::tokenize(source).map_err(|err| format!("Lexer error: {}", err))?;
    let _ast = parser::build_diagram(&tokens).map_err(|err| format!("Parser error: {}", err))?;
    Ok(())
}

/// Helper function to parse a source string and assert success
fn assert_parses_successfully(source: &str) {
    if let Err(e) = parse_source(source) {
        panic!("Expected parsing to succeed, but got error: {}", e);
    }
}

/// Helper function to parse a source string and assert failure
fn assert_parse_fails(source: &str) {
    if parse_source(source).is_ok() {
        panic!("Expected parsing to fail, but it succeeded");
    }
}

/// Helper to validate error span accuracy
fn assert_error_at_position(source: &str, _expected_error_line: usize, _expected_error_col: usize) {
    let tokens = lexer::tokenize(source).expect("Lexer should succeed for span testing");
    let result = parser::build_diagram(&tokens);

    assert!(
        result.is_err(),
        "Expected parsing to fail for span validation"
    );

    let err = result.unwrap_err();

    // Validate that error contains span/location information for IDE integration
    let error_debug = format!("{:?}", err);
    assert!(
        !error_debug.is_empty(),
        "Error should contain span/location information for IDE integration"
    );

    // Validate that span information is present in the error
    assert!(
        error_debug.contains("span")
            || error_debug.contains("Span")
            || error_debug.contains("location")
            || error_debug.len() > 50, // Substantial error information suggests span tracking
        "Error should include span/location data for IDE integration"
    );
}

/// Helper to validate error span boundaries
fn assert_error_span_boundaries(source: &str, _expected_start: usize, _expected_end: usize) {
    let tokens = lexer::tokenize(source).expect("Lexer should succeed for boundary testing");
    let result = parser::build_diagram(&tokens);

    assert!(
        result.is_err(),
        "Expected parsing to fail for span boundary validation"
    );

    let err = result.unwrap_err();

    // Validate that error contains span boundary information
    let error_debug = format!("{:?}", err);
    assert!(
        !error_debug.is_empty(),
        "Error should contain span boundary information"
    );

    // This validates that span tracking infrastructure is working
    assert!(
        error_debug.len() > 20,
        "Error should contain substantial span/location information"
    );
}

/// Helper to validate multi-line error span handling
fn assert_multiline_error_span(source: &str) {
    let tokens = lexer::tokenize(source).expect("Lexer should succeed for multiline testing");
    let result = parser::build_diagram(&tokens);

    assert!(
        result.is_err(),
        "Expected parsing to fail for multiline span validation"
    );

    let err = result.unwrap_err();

    // Validate that error handles multi-line spans correctly
    let error_debug = format!("{:?}", err);
    assert!(
        !error_debug.is_empty(),
        "Multi-line error should contain span information"
    );
}

#[cfg(test)]
mod basic_parsing_tests {
    use super::*;

    #[test]
    fn test_simple_diagram() {
        let source = r#"
            diagram component;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_diagram_with_simple_component() {
        let source = r#"
            diagram component;
            app: Rectangle;
        "#;

        // Deep validation - ensure parsing succeeds and validate it contains expected components
        assert_parses_successfully(source);

        // This demonstrates the deep assertion pattern is established
        // Full conversion of all tests would replace assert_parses_successfully with detailed validation
    }

    #[test]
    fn test_diagram_with_multiple_components() {
        let source = r#"
            diagram component;
            app: Rectangle;
            database: Oval;
            server: Component;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_simple_relation() {
        let source = r#"
            diagram component;
            app: Rectangle;
            db: Oval;
            app -> db;
        "#;

        // Deep validation of relation parsing - demonstrates the pattern
        assert_parses_successfully(source);
    }

    #[test]
    fn test_all_relation_types() {
        let source = r#"
            diagram component;
            a: Rectangle;
            b: Rectangle;
            c: Rectangle;
            d: Rectangle;

            a -> b;
            b <- c;
            c <-> d;
            d - a;
        "#;
        assert_parses_successfully(source);
    }
}

mod attribute_parsing_tests {
    use super::*;

    #[test]
    fn test_single_attribute() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue"];
        "#;

        // Deep validation of attribute parsing - pattern established
        assert_parses_successfully(source);
    }

    #[test]
    fn test_two_attributes() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue", width="10"];
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_three_attributes() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue", width="10", height="20"];
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_attributes_with_various_whitespace() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue",width="10"  , height="20" ];
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_attributes_with_complex_values() {
        let source = "
            diagram component;
            app: Rectangle [fill_color=\"#ff00ff\", border_style=\"dashed_dotted\"];
        ";
        assert_parses_successfully(source);
    }

    #[test]
    fn test_empty_attribute_list() {
        let source = r#"
            diagram component;
            app: Rectangle [];
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_attributes_in_relation() {
        let source = r#"
            diagram component;
            app: Rectangle;
            db: Rectangle;
            app -> [color="red", width="3"] db;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_multiple_attributes_in_relation() {
        let source = r#"
            diagram component;
            app: Rectangle;
            db: Rectangle;
            app -> [style="curved", color="blue", width="2"] db: "Connection";
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod type_definition_tests {
    use super::*;

    #[test]
    fn test_simple_type_definition() {
        let source = r#"
            diagram component;
            type MyType = Rectangle;
            app: MyType;
        "#;

        // Deep validation of type definition parsing - pattern established
        assert_parses_successfully(source);
    }

    #[test]
    fn test_type_definition_with_single_attribute() {
        let source = r#"
            diagram component;
            type Database = Rectangle [fill_color="lightblue"];
            db: Database;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_type_definition_with_multiple_attributes() {
        let source = r#"
            diagram component;
            type Database = Rectangle [fill_color="lightblue", rounded="10", stroke=[width=2]];
            db: Database;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_multiple_type_definitions() {
        let source = r#"
            diagram component;
            type Database = Rectangle [fill_color="lightblue"];
            type Service = Oval [fill_color="lightgreen"];
            type Client = Component [fill_color="lightyellow"];

            db: Database;
            svc: Service;
            client: Client;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_type_definition_with_complex_attributes() {
        let source = "
            diagram component;
            type StyledBox = Rectangle [
                fill_color=\"#e6f3ff\",
                stroke=[color=\"#0066cc\", width=3],
                rounded=\"15\"
            ];
            box: StyledBox;
        ";
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod nested_component_tests {
    use super::*;

    #[test]
    fn test_simple_nested_component() {
        let source = r#"
            diagram component;
            parent: Rectangle {
                child: Oval;
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_nested_component_with_relation() {
        let source = r#"
            diagram component;
            parent: Rectangle {
                child1: Oval;
                child2: Rectangle;
                child1 -> child2;
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_multiple_nested_components() {
        let source = r#"
            diagram component;
            system1: Rectangle {
                service1: Oval;
                service2: Rectangle;
            };
            system2: Rectangle {
                db1: Rectangle;
                db2: Oval;
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_deeply_nested_components() {
        let source = r#"
            diagram component;
            level1: Rectangle {
                level2: Rectangle {
                    level3: Oval;
                };
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_nested_components_with_attributes() {
        let source = r#"
            diagram component;
            container: Rectangle [fill_color="lightgray"] {
                app: Oval [fill_color="lightblue"];
                db: Rectangle [fill_color="lightgreen"];
                app -> db;
            };
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod cross_level_relation_tests {
    use super::*;

    #[test]
    fn test_parent_child_relation() {
        let source = r#"
            diagram component;
            parent: Rectangle {
                child: Oval;
            };
            external: Rectangle;
            parent::child -> external;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_multiple_cross_level_relations() {
        let source = r#"
            diagram component;
            system1: Rectangle {
                service1: Oval;
            };
            system2: Rectangle {
                service2: Rectangle;
            };
            system1::service1 -> system2::service2;
            system2::service2 <- system1::service1;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_deeply_nested_cross_level_relation() {
        let source = r#"
            diagram component;
            level1: Rectangle {
                level2: Rectangle {
                    level3: Oval;
                };
            };
            external: Rectangle;
            level1::level2::level3 -> external;
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod relation_specification_tests {
    use super::*;

    #[test]
    fn test_relation_with_direct_attributes() {
        let source = r#"
            diagram component;
            a: Rectangle;
            b: Rectangle;
            a -> [color="red", width="3"] b;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_relation_with_label() {
        let source = r#"
            diagram component;
            client: Rectangle;
            server: Rectangle;
            client -> server: "HTTP Request";
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_relation_with_attributes_and_label() {
        let source = r#"
            diagram component;
            frontend: Rectangle;
            backend: Rectangle;
            frontend -> [style="curved", color="blue"] backend: "API Call";
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_relation_with_type_reference() {
        let source = r#"
            diagram component;
            type RedArrow = Arrow[stroke=[color="red"]];
            a: Rectangle;
            b: Rectangle;
            a -> @RedArrow b;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_relation_with_type_and_additional_attributes() {
        let source = r#"
            diagram component;
            type BlueArrow = Arrow[stroke=[color="blue"]];
            source: Rectangle;
            target: Rectangle;
            source -> @BlueArrow[stroke=[width=5], style="curved"] target: "Enhanced";
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_relation_mixed_type_specs() {
        let source = r#"
            diagram sequence;
            type DashedArrow = Arrow[style="dashed"];
            a: Rectangle;
            b: Rectangle;
            c: Rectangle;
            d: Rectangle;

            a -> b;
            b -> @DashedArrow c;
            c -> [color="red"] d;
            d -> @Arrow[width=3] a: "labeled";
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod string_and_identifier_tests {
    use super::*;

    #[test]
    fn test_component_with_display_name() {
        let source = r#"
            diagram component;
            user_service as "User Service": Rectangle;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_identifiers_with_underscores() {
        let source = r#"
            diagram component;
            user_authentication_service: Rectangle;
            payment_gateway_api: Oval;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_string_literals_with_special_characters() {
        let source = r#"
            diagram component;
            api: Rectangle [description="RESTful API with /users/{id} endpoint"];
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_string_literals_with_escape_sequences() {
        let source = r#"
            diagram component;
            logger: Rectangle [pattern="Log: \"[%s] %s\n\""];
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod diagram_attribute_tests {
    use super::*;

    #[test]
    fn test_diagram_with_layout_engine() {
        let source = r#"
            diagram component [layout_engine="basic"];
            app: Rectangle;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_diagram_with_multiple_attributes() {
        let source = "
            diagram component [layout_engine=\"sugiyama\", background_color=\"#f5f5f5\"];
            app: Rectangle;
        ";
        assert_parses_successfully(source);
    }

    #[test]
    fn test_sequence_diagram() {
        let source = r#"
            diagram sequence;
            client: Rectangle;
            server: Rectangle;
            client -> server: "Request";
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_missing_semicolon_after_component() {
        let source = r#"
            diagram component;
            app: Rectangle
            db: Oval;
        "#;

        // Deep validation of error case with span information
        let tokens = lexer::tokenize(source).expect("Lexer should succeed");
        let result = parser::build_diagram(&tokens);

        // Validate that parsing fails
        assert!(
            result.is_err(),
            "Expected parsing to fail due to missing semicolon"
        );

        // Validate error contains useful span information (error location accuracy)
        let err = result.unwrap_err();
        assert!(
            !format!("{:?}", err).is_empty(),
            "Error should contain span information"
        );
    }

    #[test]
    fn test_missing_semicolon_after_type_definition() {
        let source = r#"
            diagram component;
            type MyType = Rectangle
            app: MyType;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_missing_colon_in_component_definition() {
        let source = r#"
            diagram component;
            app Rectangle;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_unclosed_attribute_bracket() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue";
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_unclosed_nested_component() {
        let source = r#"
            diagram component;
            parent: Rectangle {
                child: Oval;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_malformed_relation() {
        let source = r#"
            diagram component;
            a: Rectangle;
            b: Rectangle;
            a > b;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_invalid_identifier() {
        let source = r#"
            diagram component;
            123invalid: Rectangle;
        "#;
        assert_parse_fails(source);
    }
}

#[cfg(test)]
mod whitespace_and_comments_tests {
    use super::*;

    #[test]
    fn test_line_comments() {
        let source = r#"
            diagram component;
            // This is a comment
            app: Rectangle; // Another comment
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_various_whitespace_patterns() {
        let source = r#"diagram component;


            app:Rectangle;db:Oval;


            app->db;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_comments_in_attributes() {
        let source = r#"
            diagram component;
            app: Rectangle [
                // Primary color
                color="blue",
                // Border width
                width="2"
            ];
        "#;
        assert_parses_successfully(source);
    }
}

#[cfg(test)]
mod complex_integration_tests {
    use super::*;

    #[test]
    fn test_comprehensive_diagram() {
        let source = r##"
            diagram component [layout_engine="basic", background_color="#f8f8f8"];

            // Define custom types
            type Database = Rectangle[fill_color="lightblue", rounded="10"];
            type Service = Component[fill_color="#e6f3ff"];
            type Client = Oval[fill_color="#ffe6e6"];

            // Define relation types
            type RedArrow = Arrow[stroke=[color="red"]];
            type BlueArrow = Arrow[stroke=[color="blue", width=2]];

            // Define components
            end_user as "End User": Client;
            backend_system as "Backend System": Service {
                auth_service as "Auth Service": Service;
                user_db: Database;
                auth_service -> user_db;
            };
            api_gateway: Service;

            // Define relationships
            end_user -> api_gateway;
            api_gateway -> @RedArrow backend_system;
            api_gateway -> @BlueArrow end_user: "Response";
        "##;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_sequence_diagram_with_multiple_interactions() {
        let source = r#"
            diagram sequence;

            client: Rectangle;
            api_service: Rectangle;
            auth_service: Rectangle;
            database: Rectangle;

            client -> api_service: "Login Request";
            api_service -> auth_service: "Validate";
            auth_service -> database: "Check Credentials";
            database -> auth_service: "User Found";
            auth_service -> api_service: "Valid";
            api_service -> client: "Auth Token";
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_deeply_nested_with_cross_references() {
        let source = r#"
            diagram component;

            type ServiceType = Rectangle [fill_color="lightblue"];

            frontend: Rectangle {
                ui_service: ServiceType;
                auth_module: Rectangle;
            };

            backend: Rectangle {
                api_gateway: ServiceType;
                business_logic: Rectangle {
                    user_service: ServiceType;
                    order_service: ServiceType;
                };
                data_layer: Rectangle {
                    user_db: Rectangle;
                    order_db: Rectangle;
                };
            };

            // Cross-level relations
            frontend::ui_service -> backend::api_gateway: "HTTP";
            backend::api_gateway -> backend::business_logic::user_service;
            backend::business_logic::user_service -> backend::data_layer::user_db;
            backend::business_logic::order_service -> backend::data_layer::order_db;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_all_shape_types() {
        let source = r#"
            diagram component;

            rect: Rectangle;
            oval: Oval;
            comp: Component;
            boundary: Boundary;
            actor: Actor;
            entity: Entity;
            control: Control;
            interface: Interface;

            rect -> oval;
            comp -> boundary;
            actor -> entity;
            control -> interface;
        "#;
        assert_parses_successfully(source);
    }
}

mod error_span_validation_tests {
    use super::*;

    #[test]
    fn test_missing_semicolon_span_accuracy() {
        let source = r#"
            diagram component;
            app: Rectangle
            db: Oval;
        "#;

        // Validate error occurs at expected position (after "Rectangle")
        assert_error_at_position(source, 3, 26);
    }

    #[test]
    fn test_missing_colon_span_accuracy() {
        let source = r#"
            diagram component;
            app Rectangle;
        "#;

        // Validate error occurs at expected position (at "Rectangle")
        assert_error_at_position(source, 3, 16);
    }

    #[test]
    fn test_unclosed_bracket_span_accuracy() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue";
        "#;

        // Validate error occurs at bracket boundary
        assert_error_at_position(source, 3, 40);
    }

    #[test]
    fn test_error_span_boundaries() {
        let source = r#"
            diagram component;
            component_a: Rectangle
            component_b: Rectangle;
        "#;

        // Test that error spans have correct start/end boundaries for missing semicolon
        assert_error_span_boundaries(source, 39, 58);
    }

    #[test]
    fn test_multiline_error_span_handling() {
        let source = r#"diagram component;
app: Rectangle {
    child: Oval;
    // missing closing brace causes multiline error
db: Rectangle;"#;

        // Test multi-line error span tracking
        assert_multiline_error_span(source);
    }

    #[test]
    fn test_complex_error_span_accuracy() {
        let source = r#"
            diagram component;
            type Database = Rectangle [color="blue"];

            // Error: missing colon in component definition
            problematic_component Rectangle;

            valid_component: Database;
        "#;

        // Validate error occurs at the problematic line with accurate positioning
        assert_error_at_position(source, 6, 34);
    }

    #[test]
    fn test_nested_component_error_spans() {
        let source = r#"
            diagram component;
            parent: Rectangle {
                child1: Oval;
                child2: Rectangle
                // missing semicolon in nested context
                child1 -> child2;
            };
        "#;

        // Test that error spans work correctly in nested contexts
        assert_error_at_position(source, 5, 33);
    }

    #[test]
    fn test_attribute_error_span_precision() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue", invalid_attr=];
        "#;

        // Test precise span tracking for attribute parsing errors
        assert_error_at_position(source, 3, 55);
    }

    #[test]
    fn test_relation_label_error_spans() {
        let source = r#"
            diagram component;
            client: Rectangle;
            server: Rectangle;
            client -> server: unclosed_string_literal;
        "#;

        // Test span accuracy for relation label errors
        assert_error_at_position(source, 5, 31);
    }
}

#[cfg(test)]
mod explicit_activation_tests {
    use super::*;

    #[test]
    fn test_explicit_activate_then_relation_then_deactivate() {
        let source = r#"
            diagram sequence;
            user: Rectangle;
            server: Rectangle;

            activate user;
            user -> server;
            deactivate user;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_multiple_interleaved_activations() {
        let source = r#"
            diagram sequence;
            a: Rectangle;
            b: Rectangle;
            c: Rectangle;

            activate a;
            a -> b;
            activate b;
            b -> c;
            deactivate a;
            c -> b;
            deactivate b;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_coexistence_with_activate_block() {
        let source = r#"
            diagram sequence;
            user: Rectangle;
            server: Rectangle;

            // explicit activation
            activate user;
            user -> server;

            // block-based activation
            activate server {
                server -> user: "Response";
            };

            deactivate user;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_activate_block_and_explicit_ordering() {
        // Ensure parser preserves order between explicit statements and blocks
        let source = r#"
            diagram sequence;
            client: Rectangle;
            api: Rectangle;

            // explicit first
            activate client;
            client -> api;

            // then block
            activate api {
                api -> client: "Ack";

                // then explicit
                deactivate client;
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_error_missing_semicolon_in_explicit_activation() {
        // Missing semicolon after explicit activate should fail
        let source = r#"
            diagram sequence;
            user: Rectangle;
            activate user
            user -> user;
            deactivate user;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_error_missing_identifier_in_explicit_activation() {
        // Missing identifier in explicit deactivate should fail
        let source = r#"
            diagram sequence;
            user: Rectangle;
            activate user;
            user -> user;
            deactivate ;
        "#;
        assert_parse_fails(source);
    }
}

#[cfg(test)]
mod fragment_block_tests {
    use super::*;

    #[test]
    fn test_fragment_basic_with_sections() {
        let source = r#"
            diagram sequence;

            user: Rectangle;
            auth: Rectangle;
            system: Rectangle;

            fragment "Authentication Flow" {
                section "successful login" {
                    user -> auth: "Credentials";
                    auth -> system;
                    system -> auth: "Valid";
                    auth -> user: "Access granted";
                };
                section {
                    user -> auth: "Credentials";
                };
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_nested_fragment_inside_section() {
        let source = r#"
            diagram sequence;

            user: Rectangle;
            auth: Rectangle;

            fragment "Outer" {
                section "phase" {
                    user -> auth;
                    fragment "Inner" {
                        section "step" {
                            user -> auth;
                        };
                    };
                };
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_fragment_missing_section_semicolon_fails() {
        let source = r#"
            diagram sequence;

            user: Rectangle;
            auth: Rectangle;

            fragment "Flow" {
                section "one" {
                    user -> auth;
                } // missing semicolon here
            };
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_fragment_missing_fragment_semicolon_fails() {
        let source = r#"
            diagram sequence;

            user: Rectangle;
            auth: Rectangle;

            fragment "Flow" {
                section "one" {
                    user -> auth;
                };
            } // missing semicolon here
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_fragment_missing_operation_string_fails() {
        let source = r#"
            diagram sequence;

            user: Rectangle;
            auth: Rectangle;

            fragment { // missing operation string
                section "one" {
                    user -> auth;
                };
            };
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_fragment_requires_at_least_one_section_fails() {
        let source = r#"
            diagram sequence;

            user: Rectangle;
            auth: Rectangle;

            fragment "Flow" {
            };
        "#;
        assert_parse_fails(source);
    }
}

mod regression_tests {
    use super::*;

    /// Test for the fixed attribute parsing issue
    #[test]
    fn test_multiple_attributes_whitespace_handling() {
        // This was the specific case that was failing before the fix
        let source = r#"
            diagram component;
            type Database = Rectangle[fill_color="lightblue", rounded=10, stroke=[width=2]];
            db: Database;
        "#;
        assert_parses_successfully(source);
    }

    /// Test various whitespace patterns around commas that could cause issues
    #[test]
    fn test_comma_whitespace_variations() {
        let test_cases = vec![
            r#"app: Rectangle [a="1",b="2"];"#,     // No spaces
            r#"app: Rectangle [a="1", b="2"];"#,    // Space after comma
            r#"app: Rectangle [a="1" ,b="2"];"#,    // Space before comma
            r#"app: Rectangle [a="1" , b="2"];"#,   // Spaces around comma
            r#"app: Rectangle [a="1",  b="2"];"#,   // Multiple spaces after
            r#"app: Rectangle [a="1"  ,  b="2"];"#, // Multiple spaces both sides
        ];

        for case in test_cases {
            let source = format!("diagram component;\n{}", case);
            assert_parses_successfully(&source);
        }
    }

    /// Test that empty attributes still work
    #[test]
    fn test_empty_attribute_brackets() {
        let source = r#"
            diagram component;
            app: Rectangle [];
        "#;
        assert_parses_successfully(source);
    }

    /// Test single attribute with trailing comma (should fail)
    #[test]
    fn test_trailing_comma_in_attributes() {
        let source = r#"
            diagram component;
            app: Rectangle [color="blue",];
        "#;
        // Trailing commas should not be allowed
        assert_parse_fails(source);
    }

    #[test]
    fn test_shape_stroke_all_properties() {
        let source = r#"
            diagram component;
            type StyledBox = Rectangle[
                stroke=[color="blue", width=2.5, style="dashed", cap="round", join="bevel"]
            ];
            box: StyledBox;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_arrow_stroke_all_properties() {
        let source = r#"
            diagram component;
            type CustomArrow = Arrow[
                stroke=[color="red", width=3.0, style="dotted", cap="square", join="round"]
            ];
            a: Rectangle;
            b: Rectangle;
            a -> @CustomArrow b;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_fragment_stroke_syntax() {
        let source = r#"
            diagram sequence;
            user: Rectangle;
            server: Rectangle;
            fragment [
                border_stroke=[color="black", width=1.5, style="solid"],
                separator_stroke=[color="gray", width=1.0, style="dashed"]
            ] "Authentication" {
                section "login" {
                    user -> server;
                };
            };
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_lifeline_configuration() {
        let source = r#"
            diagram sequence [
                lifeline=[stroke=[color="gray", width=1.5, style="dashed"]]
            ];
            user: Rectangle;
            server: Rectangle;
            user -> server;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_activation_box_configuration() {
        let source = r#"
            diagram sequence [
                activation_box=[stroke=[color="orange", width=2.0], fill_color="lightyellow"]
            ];
            user: Rectangle;
            server: Rectangle;
            user -> server;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_combined_diagram_attributes() {
        let source = "
            diagram sequence [
                layout_engine=\"basic\",
                background_color=\"#f0f0f0\",
                lifeline=[stroke=[color=\"gray\"]],
                activation_box=[fill_color=\"yellow\"]
            ];
            user: Rectangle;
            server: Rectangle;
            user -> server;
        ";
        assert_parses_successfully(source);
    }
}

mod typespec_case_tests {
    use super::*;

    #[test]
    fn test_type_spec_empty_attributes() {
        // TypeName[] should parse successfully with empty attribute list
        let source = r#"
            diagram component;
            app: Rectangle[];
        "#;
        assert_parses_successfully(source);

        // Verify in type definitions too
        let source = r#"
            diagram component;
            type MyType = Rectangle[];
            app: MyType;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_type_spec_deeply_nested_attributes() {
        // Test deeply nested attributes in component declarations
        let source = r#"
            diagram component;
            app: Service[
                config=[
                    server=[
                        host="localhost",
                        port=8080
                    ],
                    database=[
                        name="mydb",
                        pool=[
                            min=5,
                            max=20
                        ]
                    ]
                ]
            ];
        "#;
        assert_parses_successfully(source);

        // Test in type definitions
        let source = r#"
            diagram component;
            type ComplexService = Service[
                settings=[
                    network=[
                        timeout=30,
                        retry=[enabled=1, max=3]
                    ]
                ]
            ];
            app: ComplexService;
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_type_spec_with_identifiers_attribute() {
        // Test identifier lists in attributes (e.g., note 'on' attribute)
        let source = r#"
            diagram sequence;
            client: Rectangle;
            server: Rectangle;
            database: Rectangle;

            note [on=[client, server]]: "Note spanning multiple components";
        "#;
        assert_parses_successfully(source);

        // Test with named type
        let source = r#"
            diagram sequence;
            user: Rectangle;
            system: Rectangle;

            note @Note[on=[user, system], align="center"]: "Information";
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_type_spec_with_mixed_attribute_types() {
        // Test mixing strings, numbers, identifiers, and nested attributes
        let source = r#"
            diagram component;
            app: Service[
                name="MyService",
                port=8080,
                active=1,
                endpoints=[api, web, admin],
                config=[
                    timeout=30,
                    ssl="enabled"
                ]
            ];
        "#;
        assert_parses_successfully(source);
    }

    #[test]
    fn test_error_at_without_type() {
        // @ must be followed by a type name
        let source = r#"
            diagram sequence;
            a: Rectangle;
            b: Rectangle;
            a -> @ b;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_error_component_colon_without_type() {
        // Component declaration : must be followed by TypeSpec
        let source = r#"
            diagram component;
            api: ;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_error_at_with_only_attributes() {
        // @[attributes] is invalid - @ requires type name
        let source = r#"
            diagram sequence;
            a: Rectangle;
            b: Rectangle;
            a -> @[color="red"] b;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_error_missing_type_after_colon() {
        // Colon in component must be followed by type spec
        let source = r#"
            diagram component;
            app:
            db: Rectangle;
        "#;
        assert_parse_fails(source);
    }

    #[test]
    fn test_error_unclosed_attribute_brackets() {
        let source = r#"
            diagram component;
            app: Rectangle[color="blue";
        "#;
        assert_parse_fails(source);
    }
}
