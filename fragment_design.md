# Fragment Data Model Design

## Overview

This document outlines the data model design for fragment blocks in Filament, covering the elaborate AST representation and sequence graph integration.

## 1. Elaborate Types Data Model

### 1.1 Core Fragment Structures

```rust
// In elaborate_types.rs

/// Represents a fragment block in a sequence diagram
#[derive(Debug)]
pub struct Fragment {
    /// The operation string (e.g., "alt", "opt", "loop", "par")
    pub operation: String,
    /// The sections within this fragment
    pub sections: Vec<FragmentSection>,
}

/// Represents a section within a fragment
#[derive(Debug)]
pub struct FragmentSection {
    /// Optional title for this section (e.g., "successful login", "failed login")
    pub title: Option<String>,
    /// Elements contained in this section
    pub elements: Vec<Element>,
}

impl Fragment {
    pub fn new(operation: String, sections: Vec<FragmentSection>) -> Self {
        Self { operation, sections }
    }
}

impl FragmentSection {
    pub fn new(title: Option<String>, elements: Vec<Element>) -> Self {
        Self { title, elements }
    }
}
```

### 1.2 Element Enum Update

```rust
// Update the Element enum to include Fragment
pub enum Element {
    Node(Node),
    Relation(Relation),
    Activate(Id),
    Deactivate(Id),
    Fragment(Fragment),  // New variant
}
```

## 2. Sequence Graph Integration

### 2.1 SequenceEvent Extension

```rust
// In sequence.rs

/// Represents ordered events in a sequence diagram
#[derive(Debug)]
pub enum SequenceEvent<'a> {
    // Existing variants
    Relation(&'a ast::Relation),
    Activate(Id),
    Deactivate(Id),
    
    // New fragment-related events
    FragmentStart {
        operation: String,
    },
    FragmentSectionStart {
        title: Option<String>,
    },
    FragmentSectionEnd,
    FragmentEnd,
}
```

### 2.2 Alternative Design: Fragment Context

```rust
// Alternative: Wrap events with fragment context
#[derive(Debug)]
pub struct FragmentContext {
    /// Stack of fragment operations we're currently within
    pub fragment_stack: Vec<String>,
    /// Current section title if within a section
    pub current_section: Option<String>,
}

// This could be attached to events or tracked separately
```

## 3. Elaboration Builder Implementation

### 3.1 Fragment Building Method

```rust
// In elaborate.rs

fn build_fragment_element(
    &mut self,
    fragment: &parser::Fragment,
    parent_id: Option<Id>,
    diagram_kind: types::DiagramKind,
) -> EResult<types::Element> {
    // Validate diagram kind
    if diagram_kind != types::DiagramKind::Sequence {
        return Err(ElaborationDiagnosticError::from_span(
            "Fragment blocks are only supported in sequence diagrams".to_string(),
            fragment.span(),
            "fragment not allowed here",
            Some("Fragment blocks are used for grouping alternatives in sequence diagrams".to_string()),
        ));
    }
    
    // Build sections recursively
    let mut sections = Vec::new();
    for parser_section in &fragment.sections {
        let elements = self.build_scope_from_elements(
            parser_section.elements.as_slice(),
            parent_id,
            diagram_kind,
        )?;
        
        sections.push(types::FragmentSection::new(
            parser_section.title.as_ref().map(|t| t.inner().to_string()),
            elements,
        ));
    }
    
    Ok(types::Element::Fragment(types::Fragment::new(
        fragment.operation.inner().to_string(),
        sections,
    )))
}
```

## 4. Sequence Graph Processing

### 4.1 Event Generation from Fragments

```rust
// In sequence.rs, within new_from_elements

ast::Element::Fragment(fragment) => {
    // Add fragment start event
    graph.add_event(SequenceEvent::FragmentStart {
        operation: fragment.operation.clone(),
    });
    
    // Process each section
    for section in &fragment.sections {
        // Add section start event
        graph.add_event(SequenceEvent::FragmentSectionStart {
            title: section.title.clone(),
        });
        
        // Recursively process elements within the section
        // This will add Relation/Activate/Deactivate events
        // and handle nested fragments
        for element in &section.elements {
            match element {
                ast::Element::Relation(relation) => {
                    graph.add_event(SequenceEvent::Relation(relation));
                }
                ast::Element::Activate(id) => {
                    graph.add_event(SequenceEvent::Activate(*id));
                }
                ast::Element::Deactivate(id) => {
                    graph.add_event(SequenceEvent::Deactivate(*id));
                }
                ast::Element::Fragment(nested_fragment) => {
                    // Recursively handle nested fragments
                    // Process using same logic
                }
                ast::Element::Node(_) => {
                    // Nodes should be defined outside fragments
                    // Could error or ignore
                }
            }
        }
        
        // Add section end event
        graph.add_event(SequenceEvent::FragmentSectionEnd);
    }
    
    // Add fragment end event
    graph.add_event(SequenceEvent::FragmentEnd);
}
```

## 5. Layout Considerations

### 5.1 Fragment Rendering in Sequence Layout

Fragments in sequence diagrams are typically rendered as:
- A labeled box surrounding the contained interactions
- Operation label in the top-left corner
- Section dividers (dashed lines) between sections
- Section titles near the dividers

```rust
// In layout/sequence.rs

/// Represents a fragment box in the layout
#[derive(Debug)]
pub struct FragmentBox {
    /// The operation string (e.g., "alt", "opt", "loop")
    pub operation: String,
    /// Y position where the fragment starts
    pub start_y: f32,
    /// Y position where the fragment ends
    pub end_y: f32,
    /// Section boundaries within the fragment
    pub sections: Vec<FragmentSectionBounds>,
    /// Nesting level (0 for top-level, increases for nested)
    pub nesting_level: u32,
}

#[derive(Debug)]
pub struct FragmentSectionBounds {
    /// Optional title for this section
    pub title: Option<String>,
    /// Y position where this section starts
    pub start_y: f32,
    /// Y position where this section ends
    pub end_y: f32,
}
```

### 5.2 Event Processing in Layout Engine

```rust
// In layout/engines/basic/sequence.rs

// Track fragment state during layout calculation
struct FragmentState {
    stack: Vec<FragmentInfo>,
}

struct FragmentInfo {
    operation: String,
    start_y: f32,
    sections: Vec<SectionInfo>,
}

struct SectionInfo {
    title: Option<String>,
    start_y: f32,
}

// Process events to build fragment boxes
let mut fragment_state = FragmentState::new();
let mut fragment_boxes = Vec::new();

for event in graph.events() {
    match event {
        SequenceEvent::FragmentStart { operation } => {
            fragment_state.push_fragment(operation.clone(), current_y);
        }
        SequenceEvent::FragmentSectionStart { title } => {
            fragment_state.push_section(title.clone(), current_y);
        }
        SequenceEvent::FragmentSectionEnd => {
            fragment_state.end_section(current_y);