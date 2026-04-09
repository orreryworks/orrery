//! File resolver: loading, dependency graph & cycle detection.
//!
//! The [`Resolver`] recursively loads `.orr` files via a
//! [`SourceProvider`], registers them in a [`SourceMap`], deduplicates by
//! canonical path, and detects circular dependencies.
//!
//! Source text is allocated in a [`bumpalo::Bump`] arena so that all
//! [`FileAst`] nodes can borrow from it with a single `'arena` lifetime.
//!
//! # Overview
//!
//! - [`Resolver`] — entry point that drives recursive file loading.
//! - [`ResolvedFile`] — the output: a root [`FileAst`] with its
//!   [`imports`](FileAst::imports) populated recursively, plus a
//!   [`SourceMap`] covering every loaded file.
//!
//! # Data Flow
//!
//! ```text
//! root path
//!     ↓ Resolver::resolve
//! recursive resolve_file
//!     ├─ SourceProvider::read_source  → source text
//!     ├─ Bump::alloc_str             → arena-allocated &str
//!     ├─ SourceMap::add_file         → virtual byte offset
//!     ├─ lexer::tokenize             → tokens
//!     ├─ parser::build_file          → FileAst
//!     └─ recurse for each import     → populate FileAst.imports
//!     ↓
//! ResolvedFile { source_map, file_ast }
//! ```

use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use bumpalo::Bump;

use crate::{
    error::{Diagnostic, ErrorCode, ParseError, SourceError},
    file_id::FileId,
    lexer, parser,
    parser_types::{FileAst, Import, ImportDecl, ImportForm},
    source_map::SourceMap,
    source_provider::SourceProvider,
    span::{Span, Spanned},
};

/// The output of resolving a root Orrery file and all of its transitive
/// imports.
///
/// The [`file_ast`](ResolvedFile::file_ast) has its
/// [`imports`](FileAst::imports) field populated recursively — each imported
/// file in turn has *its* imports populated, forming a tree that mirrors the
/// import graph.
///
/// The [`source_map`](ResolvedFile::source_map) covers every file that was
/// loaded during resolution, enabling span lookups across the entire tree.
#[derive(Debug)]
pub struct ResolvedFile<'arena> {
    source_map: SourceMap<'arena>,
    file_ast: FileAst<'arena>,
}

impl<'arena> ResolvedFile<'arena> {
    /// Consumes the resolved file and returns the AST and source map
    /// as separate values.
    pub fn into_parts(self) -> (FileAst<'arena>, SourceMap<'arena>) {
        (self.file_ast, self.source_map)
    }
}

/// Recursively resolves Orrery files, builds import trees, and detects
/// cycles.
///
/// The resolver is consumed by [`Resolver::resolve`], which returns a
/// [`ResolvedFile`] on success.
///
/// # Type parameters
///
/// * `'arena` — lifetime of the [`Bump`] arena that holds source text.
/// * `P` — the [`SourceProvider`] implementation used for file resolution
///   and reading.
///
/// # Examples
///
/// ```text
/// let arena = Bump::new();
/// let provider = MySourceProvider::new();
/// let resolver = Resolver::new(&arena, provider);
/// let resolved = resolver.resolve(Path::new("main.orr"))?;
/// ```
pub struct Resolver<'arena, P> {
    arena: &'arena Bump,
    provider: P,
    source_map: SourceMap<'arena>,
    /// Caches resolved [`FileAst`]s by [`FileId`] so each file is read
    /// and parsed at most once. Diamond dependencies share the same
    /// `Rc<RefCell<FileAst>>` instance.
    cache: HashMap<FileId, Rc<RefCell<FileAst<'arena>>>>,
    /// [`FileId`]s currently being resolved, used for cycle detection.
    resolution_stack: Vec<FileId>,
}

impl<'arena, P: SourceProvider> Resolver<'arena, P> {
    /// Creates a new resolver backed by the given arena and source provider.
    ///
    /// # Arguments
    ///
    /// * `arena` — [`Bump`] arena used to store source text for the `'arena`
    ///   lifetime.
    /// * `provider` — source provider for resolving and reading Orrery files.
    pub fn new(arena: &'arena Bump, provider: P) -> Self {
        Self {
            arena,
            provider,
            source_map: SourceMap::new(),
            cache: HashMap::new(),
            resolution_stack: Vec::new(),
        }
    }

    /// Resolves the root file at `root_path` and all of its transitive
    /// imports.
    ///
    /// The returned [`ResolvedFile`] contains the root [`FileAst`] with its
    /// [`imports`](FileAst::imports) field populated recursively, plus a
    /// [`SourceMap`] covering every loaded file.
    ///
    /// # Arguments
    ///
    /// * `root_path` — path to the root/entry Orrery file.
    ///
    /// # Returns
    ///
    /// A [`ResolvedFile`] containing the source map and the fully resolved
    /// root AST.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if any file cannot be found (E400), a circular
    /// dependency is detected (E401), an import path is invalid (E402), or
    /// lexing/parsing fails.
    pub fn resolve(mut self, root_path: &Path) -> Result<ResolvedFile<'arena>, ParseError<'arena>> {
        let root_rc = match self.resolve_file(root_path, None) {
            Ok(rc) => rc,
            Err(diags) => return Err(ParseError::new(diags, self.source_map)),
        };

        // Drop the cache so the root Rc's refcount drops to 1.
        // (Imported files inside the tree keep their own Rc clones alive.)
        self.cache.clear();

        let file_ast = Rc::try_unwrap(root_rc)
            .expect("cache cleared; root refcount should be 1")
            .into_inner();

        Ok(ResolvedFile {
            source_map: self.source_map,
            file_ast,
        })
    }

    /// Recursively resolves a single file and all of its imports.
    ///
    /// Returns an `Rc<RefCell<FileAst>>` with its
    /// [`imports`](FileAst::imports) field populated. If the file was already
    /// parsed (diamond dependency), the cached `Rc` is cloned (cheap).
    fn resolve_file(
        &mut self,
        path: &Path,
        import_span: Option<Span>,
    ) -> Result<Rc<RefCell<FileAst<'arena>>>, Vec<Diagnostic>> {
        let file_id = FileId::new(path);

        // 1. Deduplication — return cached Rc if already fully resolved.
        if let Some(rc) = self.cache.get(&file_id) {
            return Ok(Rc::clone(rc));
        }

        // 2. Cycle detection.
        if self.resolution_stack.contains(&file_id) {
            return Err(vec![self.cycle_error(file_id, import_span)]);
        }

        // 3. Push onto resolution stack.
        self.resolution_stack.push(file_id);

        // 4. Read source text from the provider.
        let source_string = self
            .provider
            .read_source(path)
            .map_err(|e| vec![Self::file_not_found_diagnostic(&e, import_span)])?;

        // 5. Copy source text into the arena (long-lived allocation).
        let source: &'arena str = self.arena.alloc_str(&source_string);
        drop(source_string);

        // 6. Register in the source map.
        let base_offset = self
            .source_map
            .add_file(path.display().to_string(), source, import_span);

        // 7. Tokenize.
        let tokens = lexer::tokenize(source, base_offset)?;

        // 8. Parse.
        let mut file_ast = parser::build_file(&tokens).map_err(|diag| vec![diag])?;

        // 9. Resolve each import declaration and populate `file_ast.imports`.
        for import_decl in &file_ast.import_decls {
            let import = self.resolve_import_decl(path, import_decl)?;
            file_ast.imports.push(import);
        }

        // 10. Pop from the resolution stack and cache the result.
        self.resolution_stack.pop();
        let rc = Rc::new(RefCell::new(file_ast));
        self.cache.insert(file_id, Rc::clone(&rc));

        Ok(rc)
    }

    /// Resolves a single [`ImportDecl`] into a fully populated [`Import`].
    ///
    /// Validates the path, recursively resolves the referenced file, and
    /// derives the namespace (for namespaced imports) or leaves it `None`
    /// (for glob imports).
    ///
    /// # Errors
    ///
    /// Returns diagnostics if:
    /// - The referenced file cannot be found (E400).
    /// - A circular dependency is detected (E401).
    /// - The import path is empty (E402).
    /// - The namespace cannot be derived from the file path (E403,
    ///   namespaced imports only).
    fn resolve_import_decl(
        &mut self,
        parent_path: &Path,
        import_decl: &Spanned<ImportDecl>,
    ) -> Result<Import<'arena>, Vec<Diagnostic>> {
        let import_path = &import_decl.path;
        let decl_span = import_decl.span();

        Self::validate_import_path(import_path, decl_span).map_err(|diag| vec![diag])?;

        let resolved_path = self
            .provider
            .resolve_path(parent_path, import_path)
            .map_err(|e| vec![Self::file_not_found_diagnostic(&e, Some(decl_span))])?;

        let file_ast = self.resolve_file(&resolved_path, Some(decl_span))?;

        let namespace = match import_decl.inner().form {
            ImportForm::Namespaced => {
                let ns = self
                    .provider
                    .derive_namespace(&resolved_path)
                    .map_err(|e| vec![Self::invalid_namespace_diagnostic(&e, decl_span)])?;
                Some(ns)
            }
            ImportForm::Glob => None,
        };

        Ok(Import {
            namespace,
            file_ast,
        })
    }

    /// Rejects empty import paths, which would silently mis-resolve to the
    /// parent directory with an `.orr` extension.
    fn validate_import_path(import_path: &str, span: Span) -> Result<(), Diagnostic> {
        if import_path.is_empty() {
            return Err(Diagnostic::error("import path is empty")
                .with_code(ErrorCode::E402)
                .with_label(span, "empty path"));
        }

        Ok(())
    }

    /// Builds a [`Diagnostic`] for a namespace-derivation failure (E403).
    fn invalid_namespace_diagnostic(source_error: &SourceError, span: Span) -> Diagnostic {
        Diagnostic::error(format!(
            "cannot derive namespace: {}",
            source_error.message()
        ))
        .with_code(ErrorCode::E403)
        .with_label(span, "imported here")
    }

    /// Builds a [`Diagnostic`] for a file-not-found failure (E400).
    fn file_not_found_diagnostic(source_error: &SourceError, span: Option<Span>) -> Diagnostic {
        let mut diag = Diagnostic::error(format!(
            "cannot find file: {}",
            source_error.path().display()
        ))
        .with_code(ErrorCode::E400);

        if let Some(span) = span {
            diag = diag.with_label(span, "imported here");
        }

        diag
    }

    /// Builds a circular-dependency [`Diagnostic`] (E401) showing the full
    /// import chain.
    fn cycle_error(&self, file_id: FileId, import_span: Option<Span>) -> Diagnostic {
        let cycle_start = self
            .resolution_stack
            .iter()
            .position(|fid| *fid == file_id)
            .expect("cycle target must be on the resolution stack");

        let chain: Vec<&str> = self.resolution_stack[cycle_start..]
            .iter()
            .map(|fid| fid.as_str())
            .chain(std::iter::once(file_id.as_str()))
            .collect();
        let chain_str = chain.join(" → ");

        let mut diag = Diagnostic::error("circular dependency detected")
            .with_code(ErrorCode::E401)
            .with_help(format!("import chain: {chain_str}"));

        if let Some(span) = import_span {
            diag = diag.with_label(span, "cyclic import");
        }

        diag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser_types::FileHeader, source_provider::InMemorySourceProvider};

    /// Helper: create a provider with the given files and resolve from `root`.
    fn resolve_with<'a>(
        arena: &'a Bump,
        files: &[(&str, &str)],
        root: &str,
    ) -> Result<ResolvedFile<'a>, ParseError<'a>> {
        let mut provider = InMemorySourceProvider::new();
        for &(path, source) in files {
            provider.add_file(path, source);
        }
        let resolver = Resolver::new(arena, provider);
        resolver.resolve(Path::new(root))
    }

    #[test]
    fn single_file_no_imports() {
        let arena = Bump::new();
        let resolved = resolve_with(
            &arena,
            &[("main.orr", "diagram component;\na: Rectangle;")],
            "main.orr",
        )
        .expect("should resolve");

        assert!(resolved.file_ast.imports.is_empty());
        assert_eq!(resolved.source_map.file_count(), 1);
    }

    #[test]
    fn library_file_resolves() {
        let arena = Bump::new();
        let resolved = resolve_with(&arena, &[("lib.orr", "library;")], "lib.orr")
            .expect("should resolve library");

        assert!(resolved.file_ast.imports.is_empty());
        assert!(matches!(
            resolved.file_ast.header,
            FileHeader::Library { .. }
        ));
    }

    #[test]
    fn single_import_loads_both() {
        let arena = Bump::new();
        let resolved = resolve_with(
            &arena,
            &[
                (
                    "main.orr",
                    "diagram component;\nimport \"styles\";\na: Rectangle;",
                ),
                ("styles.orr", "library;"),
            ],
            "main.orr",
        )
        .expect("should resolve");

        assert_eq!(resolved.file_ast.imports.len(), 1);
        assert_eq!(resolved.source_map.file_count(), 2);

        // The imported file should be a library.
        let imported = &resolved.file_ast.imports[0];
        let imported_ast = imported.file_ast.borrow();
        assert!(matches!(imported_ast.header, FileHeader::Library { .. }));
    }

    #[test]
    fn multiple_imports_from_same_file() {
        // A imports B, C, and D; no shared deps.
        let arena = Bump::new();
        let resolved = resolve_with(
                &arena,
                &[
                    (
                        "a.orr",
                        "diagram component;\nimport \"b\";\nimport \"c\";\nimport \"d\";\na: Rectangle;",
                    ),
                    ("b.orr", "library;"),
                    ("c.orr", "library;"),
                    ("d.orr", "library;"),
                ],
                "a.orr",
            )
            .expect("should resolve");

        assert_eq!(resolved.file_ast.imports.len(), 3);
        assert_eq!(resolved.source_map.file_count(), 4);
    }

    #[test]
    fn nested_imports_resolve_transitively() {
        // A → B → C
        let arena = Bump::new();
        let resolved = resolve_with(
            &arena,
            &[
                ("a.orr", "diagram component;\nimport \"b\";\na: Rectangle;"),
                ("b.orr", "library;\nimport \"c\";"),
                ("c.orr", "library;"),
            ],
            "a.orr",
        )
        .expect("should resolve nested imports");

        assert_eq!(resolved.source_map.file_count(), 3);

        // A → B → C chain.
        let b_ast = resolved.file_ast.imports[0].file_ast.borrow();
        let c_ast = b_ast.imports[0].file_ast.borrow();
        assert!(matches!(c_ast.header, FileHeader::Library { .. }));
        assert!(c_ast.imports.is_empty());
    }

    #[test]
    fn duplicate_import_in_same_file_deduplicates() {
        // A imports B twice — B should be parsed only once.
        let arena = Bump::new();
        let resolved = resolve_with(
            &arena,
            &[
                (
                    "a.orr",
                    "diagram component;\nimport \"b\";\nimport \"b\";\na: Rectangle;",
                ),
                ("b.orr", "library;"),
            ],
            "a.orr",
        )
        .expect("should resolve");

        // Two import entries in the AST (one per import decl).
        assert_eq!(resolved.file_ast.imports.len(), 2);

        // But only 2 files in the source map (not 3).
        assert_eq!(resolved.source_map.file_count(), 2);
    }

    #[test]
    fn diamond_dependency_deduplicates() {
        // A imports B and C; both B and C import D.
        let arena = Bump::new();
        let resolved = resolve_with(
            &arena,
            &[
                (
                    "a.orr",
                    "diagram component;\nimport \"b\";\nimport \"c\";\na: Rectangle;",
                ),
                ("b.orr", "library;\nimport \"d\";"),
                ("c.orr", "library;\nimport \"d\";"),
                ("d.orr", "library;"),
            ],
            "a.orr",
        )
        .expect("should resolve diamond");

        // D is loaded only once (source map has 4 entries, not 5).
        assert_eq!(resolved.source_map.file_count(), 4);

        // A has two imports (B and C).
        assert_eq!(resolved.file_ast.imports.len(), 2);

        // Both B and C each have one import (D).
        let b_ast = resolved.file_ast.imports[0].file_ast.borrow();
        let c_ast = resolved.file_ast.imports[1].file_ast.borrow();
        assert_eq!(b_ast.imports.len(), 1);
        assert_eq!(c_ast.imports.len(), 1);
    }

    #[test]
    fn missing_root_file_emits_e400() {
        let arena = Bump::new();
        let result = resolve_with(&arena, &[], "missing.orr");

        let err = result.expect_err("should fail on missing root");
        let diag = &err.diagnostics()[0];
        assert_eq!(
            diag.code().expect("should have error code"),
            ErrorCode::E400
        );
    }

    #[test]
    fn missing_file_emits_e400() {
        let arena = Bump::new();
        let result = resolve_with(
            &arena,
            &[("main.orr", "diagram component;\nimport \"nonexistent\";")],
            "main.orr",
        );

        let err = result.expect_err("should fail on missing file");
        let diag = &err.diagnostics()[0];
        assert_eq!(
            diag.code().expect("should have error code"),
            ErrorCode::E400
        );
    }

    #[test]
    fn self_import_cycle_emits_e401() {
        // A imports itself.
        let arena = Bump::new();
        let result = resolve_with(
            &arena,
            &[("a.orr", "diagram component;\nimport \"a\";")],
            "a.orr",
        );

        let err = result.expect_err("should detect self-cycle");
        let diag = &err.diagnostics()[0];
        assert_eq!(
            diag.code().expect("should have error code"),
            ErrorCode::E401
        );
    }

    #[test]
    fn circular_dependency_emits_e401() {
        // A → B → C → A
        let arena = Bump::new();
        let result = resolve_with(
            &arena,
            &[
                ("a.orr", "diagram component;\nimport \"b\";"),
                ("b.orr", "library;\nimport \"c\";"),
                ("c.orr", "library;\nimport \"a\";"),
            ],
            "a.orr",
        );

        let err = result.expect_err("should detect 3-level cycle");
        let diag = &err.diagnostics()[0];
        assert_eq!(
            diag.code().expect("should have error code"),
            ErrorCode::E401
        );

        // The help message should show the full import chain.
        let help = diag.help().expect("should have help text");
        assert!(help.contains("a.orr"), "chain should mention a.orr: {help}");
        assert!(help.contains("b.orr"), "chain should mention b.orr: {help}");
        assert!(help.contains("c.orr"), "chain should mention c.orr: {help}");
    }

    #[test]
    fn namespace_derived_from_import_path() {
        let arena = Bump::new();
        let resolved = resolve_with(
            &arena,
            &[
                (
                    "dir/main.orr",
                    "diagram component;\nimport \"../common/base\";\na: Rectangle;",
                ),
                ("dir/../common/base.orr", "library;"),
            ],
            "dir/main.orr",
        )
        .expect("should resolve relative import");

        let import = &resolved.file_ast.imports[0];
        let ns = import.namespace.as_ref().expect("should have namespace");
        assert!(ns == "base", "expected namespace 'base', got '{ns:?}'");
    }

    #[test]
    fn cycle_error_shows_readable_paths() {
        // A → B → A (simple cycle)
        let arena = Bump::new();
        let result = resolve_with(
            &arena,
            &[
                ("src/app.orr", "diagram component;\nimport \"lib\";"),
                ("src/lib.orr", "library;\nimport \"app\";"),
            ],
            "src/app.orr",
        );

        let err = result.expect_err("should detect cycle");
        let diag = &err.diagnostics()[0];
        assert_eq!(
            diag.code().expect("should have error code"),
            ErrorCode::E401
        );

        let help = diag.help().expect("should have help text");
        // The help message must contain actual file paths, not opaque identifiers.
        assert!(
            help.contains("src/app.orr"),
            "help should show readable path 'src/app.orr': {help}"
        );
        assert!(
            help.contains("src/lib.orr"),
            "help should show readable path 'src/lib.orr': {help}"
        );
        // Paths should appear as a readable chain with arrow separators.
        assert!(
            help.contains(" → "),
            "help should format the chain with arrow separators: {help}"
        );
    }
}
