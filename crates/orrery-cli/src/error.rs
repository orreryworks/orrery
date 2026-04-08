//! Error types and miette diagnostic rendering for the Orrery CLI.
//!
//! [`Error`] is the top-level error type returned by [`crate::run()`]. It
//! wraps either a parse error (with rich source-location diagnostics) or
//! a render pipeline error.
//!
//! The [`Error::reportables()`] method converts an error into individually
//! renderable miette diagnostics. For parse errors that contain multiple
//! diagnostics, each one becomes a separate reportable item.
//!
//! # Multi-File Support
//!
//! [`SourceMapSource`] wraps a [`SourceMap`] reference and implements
//! [`miette::SourceCode`], translating virtual-space spans back to the
//! correct file. This enables miette to render source snippets from any
//! file in the import tree.
//!
//! # Import Traces
//!
//! When an error occurs in an imported file, import trace diagnostics are
//! built from primary label spans and exposed through miette's `related()`
//! method so the import location is visible in the rendered output.

use std::{fmt, iter};

use miette::{
    Diagnostic as MietteDiagnostic, LabeledSpan, MietteError, MietteSpanContents, SourceCode,
    SourceSpan, SpanContents,
};

use orrery::RenderError;
use orrery_parser::{
    Span,
    error::{Diagnostic, ParseError},
    source_map::SourceMap,
};

/// Errors that can occur during CLI execution.
///
/// Separates parse errors (which carry a [`SourceMap`] for rich multi-file
/// diagnostics) from render pipeline errors (I/O, layout, export).
#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    /// A parse error with structured diagnostics and a source map.
    #[error("{0}")]
    Parse(ParseError<'a>),
    /// A render pipeline error (I/O, graph, layout, or export).
    #[error("{0}")]
    Render(#[from] RenderError),
}

impl<'a> From<ParseError<'a>> for Error<'a> {
    fn from(err: ParseError<'a>) -> Self {
        Self::Parse(err)
    }
}

impl From<std::io::Error> for Error<'_> {
    fn from(err: std::io::Error) -> Self {
        Self::Render(RenderError::Io(err))
    }
}

impl<'a> Error<'a> {
    /// Convert this error into individually renderable miette diagnostics.
    ///
    /// For [`Error::Parse`], returns one reportable per [`Diagnostic`] in the
    /// error, each backed by the shared [`SourceMap`] for multi-file snippet
    /// rendering and import traces.
    ///
    /// For [`Error::Render`], returns a single reportable.
    pub fn reportables(&'a self) -> Vec<Box<dyn MietteDiagnostic + 'a>> {
        match self {
            Error::Parse(parse_err) => {
                let source_map = parse_err.source_map();
                parse_err
                    .diagnostics()
                    .iter()
                    .map(|d| {
                        Box::new(DiagnosticAdapter::new(d, source_map)) as Box<dyn MietteDiagnostic>
                    })
                    .collect()
            }
            Error::Render(render_err) => {
                vec![Box::new(RenderErrorAdapter(render_err)) as Box<dyn MietteDiagnostic>]
            }
        }
    }
}

/// Newtype wrapper around [`SourceMap`] that implements [`miette::SourceCode`].
///
/// Required because the orphan rule prevents implementing a foreign trait
/// (`miette::SourceCode`) on a foreign type (`SourceMap`) directly.
///
/// The implementation translates a virtual-space [`SourceSpan`] to the
/// owning file, computes a local offset, and delegates to the file's source
/// text wrapped in a [`miette::NamedSource`].
#[derive(Debug, Clone, Copy)]
struct SourceMapSource<'a>(&'a SourceMap<'a>);

impl SourceCode for SourceMapSource<'_> {
    fn read_span<'s>(
        &'s self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'s> + 's>, MietteError> {
        let offset = span.offset();

        let file = self.0.lookup_file(offset).ok_or(MietteError::OutOfBounds)?;

        // Translate virtual offset → local offset within the file.
        let local_offset = offset - file.start_offset();
        let local_span = SourceSpan::new(local_offset.into(), span.len());

        // Delegate context-line extraction to the existing `&str` impl.
        let contents =
            file.source()
                .read_span(&local_span, context_lines_before, context_lines_after)?;

        // Translate the local span back to virtual coordinates so that
        // miette can match label offsets (which are virtual) against the
        // returned data range.
        let local = contents.span();
        let virtual_span =
            SourceSpan::new((local.offset() + file.start_offset()).into(), local.len());

        // Re-wrap with the file name so miette prints it in the header.
        Ok(Box::new(MietteSpanContents::new_named(
            file.name().to_owned(),
            contents.data(),
            virtual_span,
            contents.line(),
            contents.column(),
            contents.line_count(),
        )))
    }
}

/// Adapter for a single orrery [`Diagnostic`].
///
/// Wraps a [`Diagnostic`] together with the [`SourceMap`] so that miette
/// can render source snippets from the correct file and display the import
/// trace chain via [`related()`](MietteDiagnostic::related).
#[derive(thiserror::Error)]
#[error("{}", .diag.message())]
struct DiagnosticAdapter<'a> {
    diag: &'a Diagnostic,
    source_code: SourceMapSource<'a>,
    imports: Vec<ImportDiagnostic<'a>>,
}

impl<'a> DiagnosticAdapter<'a> {
    fn new(diag: &'a Diagnostic, source_map: &'a SourceMap<'a>) -> Self {
        let imports = Self::build_imports(diag, source_map);
        Self {
            diag,
            source_code: SourceMapSource(source_map),
            imports,
        }
    }

    /// Build [`ImportDiagnostic`]s for primary labels that fall in imported files.
    ///
    /// For each primary label, looks up its file and extracts the
    /// `first_imported_at` span — the `import "…";` declaration in the
    /// parent file. Root-file labels (where `first_imported_at` is `None`)
    /// are skipped.
    fn build_imports(
        diag: &Diagnostic,
        source_map: &'a SourceMap<'a>,
    ) -> Vec<ImportDiagnostic<'a>> {
        diag.labels()
            .iter()
            .filter(|l| l.is_primary())
            .filter_map(|l| {
                source_map
                    .lookup_file_by_span(l.span())
                    .and_then(|f| f.first_imported_at())
            })
            .map(|import_span| ImportDiagnostic::new(import_span, source_map))
            .collect()
    }
}

impl fmt::Debug for DiagnosticAdapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiagnosticAdapter")
            .field("diag", &self.diag)
            .finish()
    }
}

impl MietteDiagnostic for DiagnosticAdapter<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.diag
            .code()
            .map(|c| Box::new(c) as Box<dyn fmt::Display>)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.diag
            .help()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source_code as &dyn miette::SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let labels = self.diag.labels();
        if labels.is_empty() {
            return None;
        }

        Some(Box::new(labels.iter().map(|label| {
            let span = span_to_miette(label.span());
            let message = Some(label.message().to_string());
            if label.is_primary() {
                LabeledSpan::new_primary_with_span(message, span)
            } else {
                LabeledSpan::new_with_span(message, span)
            }
        })))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn MietteDiagnostic> + 'a>> {
        if self.imports.is_empty() {
            return None;
        }
        Some(Box::new(
            self.imports
                .iter()
                .map(|import| import as &dyn MietteDiagnostic),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
struct ImportDiagnostic<'a> {
    span: Span,
    source_code: SourceMapSource<'a>,
    next: Option<Box<ImportDiagnostic<'a>>>,
}

impl<'a> ImportDiagnostic<'a> {
    fn new(span: Span, source_map: &'a SourceMap<'a>) -> Self {
        let next = source_map
            .lookup_file_by_span(span)
            .and_then(|f| f.first_imported_at())
            .map(|import_span| Box::new(ImportDiagnostic::new(import_span, source_map)));
        ImportDiagnostic {
            span,
            source_code: SourceMapSource(source_map),
            next,
        }
    }
}

impl fmt::Display for ImportDiagnostic<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "import trace")
    }
}

impl MietteDiagnostic for ImportDiagnostic<'_> {
    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source_code as &dyn SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let span = span_to_miette(self.span);
        Some(Box::new(iter::once(LabeledSpan::new_with_span(
            Some("imported here".to_string()),
            span,
        ))))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn MietteDiagnostic> + 'a>> {
        let next = self.next.as_ref()?;
        Some(Box::new(iter::once(next.as_ref() as &dyn MietteDiagnostic)))
    }
}

/// Adapter for [`RenderError`] variants so they can be rendered by miette.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
struct RenderErrorAdapter<'a>(&'a RenderError);

impl MietteDiagnostic for RenderErrorAdapter<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        let code = match &self.0 {
            RenderError::Io(_) => "orrery::io",
            RenderError::Graph(_) => "orrery::graph",
            RenderError::Layout(_) => "orrery::layout",
            RenderError::Export(_) => "orrery::export",
        };
        Some(Box::new(code))
    }
}

/// Convert an orrery [`Span`] to a miette [`SourceSpan`].
fn span_to_miette(span: Span) -> SourceSpan {
    SourceSpan::new(span.start().into(), span.len())
}

#[cfg(test)]
mod tests {
    use orrery_parser::error::ErrorCode;

    use super::*;

    /// Helper: build a SourceMap with a single file.
    fn single_file_source_map<'a>(name: &str, source: &'a str) -> SourceMap<'a> {
        let mut sm = SourceMap::new();
        sm.add_file(name, source, None);
        sm
    }

    #[test]
    fn test_single_diagnostic_with_source_map() {
        let source = "hello";
        let sm = single_file_source_map("test.orr", source);
        let diag = Diagnostic::error("test error")
            .with_code(ErrorCode::E300)
            .with_label(Span::new(0..5), "here")
            .with_help("try this");
        let parse_err = ParseError::new(vec![diag], sm);
        let err = Error::Parse(parse_err);

        let reportables = err.reportables();
        assert_eq!(reportables.len(), 1);
        assert_eq!(reportables[0].to_string(), "test error");
    }

    #[test]
    fn test_multiple_diagnostics() {
        let source = "source code that is long enough for spans";
        let sm = single_file_source_map("test.orr", source);
        let diags = vec![
            Diagnostic::error("first error")
                .with_code(ErrorCode::E300)
                .with_label(Span::new(0..5), "first"),
            Diagnostic::error("second error")
                .with_code(ErrorCode::E301)
                .with_label(Span::new(10..15), "second")
                .with_help("help for second"),
            Diagnostic::error("third error").with_label(Span::new(20..25), "third"),
        ];
        let parse_err = ParseError::new(diags, sm);
        let err = Error::Parse(parse_err);

        let reportables = err.reportables();

        assert_eq!(reportables.len(), 3);
        assert_eq!(reportables[0].to_string(), "first error");
        assert_eq!(reportables[1].to_string(), "second error");
        assert_eq!(reportables[2].to_string(), "third error");
    }

    #[test]
    fn test_single_render_error() {
        let err = Error::Render(RenderError::Graph("graph error".to_string()));

        let reportables = err.reportables();

        assert_eq!(reportables.len(), 1);
        assert_eq!(reportables[0].to_string(), "Graph error: graph error");
    }

    #[test]
    fn test_all_labels_returned() {
        let source = "some source code text";
        let sm = single_file_source_map("test.orr", source);
        let diag = Diagnostic::error("error with labels")
            .with_label(Span::new(0..5), "primary label")
            .with_secondary_label(Span::new(10..15), "secondary label");

        let adapter = DiagnosticAdapter::new(&diag, &sm);

        let labels: Vec<_> = adapter.labels().unwrap().collect();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].label(), Some("primary label"));
        assert_eq!(labels[1].label(), Some("secondary label"));
    }

    #[test]
    fn test_primary_flag_on_labels() {
        let source = "some source code text";
        let sm = single_file_source_map("test.orr", source);
        let diag = Diagnostic::error("error with labels")
            .with_label(Span::new(0..5), "primary")
            .with_secondary_label(Span::new(10..15), "secondary");

        let adapter = DiagnosticAdapter::new(&diag, &sm);

        let labels: Vec<_> = adapter.labels().unwrap().collect();
        assert_eq!(labels.len(), 2);
        assert!(labels[0].primary());
        assert!(!labels[1].primary());
    }

    #[test]
    fn test_source_map_as_source_code() {
        use miette::SourceCode;

        let source = "line one\nline two\nline three";
        let sm = single_file_source_map("main.orr", source);
        let src = SourceMapSource(&sm);

        // Read a span in the middle of the source.
        let span = SourceSpan::new(9.into(), 8); // "line two"
        let contents = src
            .read_span(&span, 0, 0)
            .expect("read_span should succeed");
        let text = std::str::from_utf8(contents.data()).unwrap();
        assert!(text.contains("line two"));
    }

    #[test]
    fn test_source_map_multi_file() {
        use miette::SourceCode;

        let mut sm = SourceMap::new();
        let _base_a = sm.add_file("a.orr", "aaaa", None);
        let base_b = sm.add_file("b.orr", "bbbb", Some(Span::new(0..4)));
        let src = SourceMapSource(&sm);

        // Read from the second file.
        let span = SourceSpan::new(base_b.into(), 4);
        let contents = src
            .read_span(&span, 0, 0)
            .expect("read_span should succeed");
        let name = contents.name().expect("should have a file name");
        assert_eq!(name, "b.orr");
    }

    #[test]
    fn test_import_trace_root_file_no_related() {
        let source = "diagram component;\nbox: Rectangle;";
        let sm = single_file_source_map("main.orr", source);
        let diag = Diagnostic::error("error in root").with_label(Span::new(0..7), "here");

        let adapter = DiagnosticAdapter::new(&diag, &sm);

        // Root file → no import trace → related() returns None.
        assert!(adapter.related().is_none());
    }

    #[test]
    fn test_import_trace_imported_file_has_related() {
        let mut sm = SourceMap::new();
        // Root file: "import \"lib\";\n" (14 bytes)
        let _base_root = sm.add_file("main.orr", "import \"lib\";\n", None);
        // Imported file starts after root + 1-byte gap
        let base_lib = sm.add_file("lib.orr", "library;\ntype Bad;", Some(Span::new(0..13)));

        // Error in the imported file (e.g., at offset base_lib..base_lib+7 = "library")
        let diag = Diagnostic::error("error in lib")
            .with_label(Span::new(base_lib..(base_lib + 7)), "here");

        let adapter = DiagnosticAdapter::new(&diag, &sm);

        // Should have one related diagnostic showing the import chain.
        let related: Vec<_> = adapter.related().unwrap().collect();
        assert_eq!(related.len(), 1);
        assert!(related[0].labels().is_some());
    }
}
