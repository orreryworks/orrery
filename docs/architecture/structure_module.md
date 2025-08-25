# Structure Module Architecture

## Overview

The `structure` module provides the core graph data structures for representing Filament diagrams after parsing and elaboration. It replaces the previous petgraph-based implementation with a custom, lightweight graph structure that is better optimized for Filament's specific needs.

## Design Decisions

### Why Custom Implementation?

1. **Memory Efficiency**: Eliminated petgraph dependency and its overhead
2. **Type Safety**: Stronger compile-time guarantees with lifetime tracking
3. **Domain-Specific**: Optimized for Filament's specific graph patterns
4. **Simpler API**: Cleaner interface tailored to diagram needs

### Lifetime Management

The module uses Rust's lifetime system to ensure:
- Edge indices cannot outlive their graphs
- References to AST nodes remain valid
- No use-after-free bugs during graph traversal

### Separation of Concerns

Component and sequence graphs are intentionally separate because:
- They have fundamentally different structures (hierarchical vs. temporal)
- They require different traversal patterns
- They optimize for different operations
