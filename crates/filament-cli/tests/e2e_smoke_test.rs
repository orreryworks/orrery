use std::{fs, path::PathBuf};

use tempfile::tempdir;

use filament_cli::{Args, run};

/// Collects all .fil files from a directory
fn collect_fil_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut files = if let Ok(entries) = fs::read_dir(&dir) {
        entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fil")
            })
            .collect()
    } else {
        Vec::new()
    };

    // Sort for consistent test output
    files.sort();
    files
}

#[test]
fn e2e_smoke_test_valid_examples() {
    // Create a temporary directory for test outputs
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Examples are at workspace root, relative to workspace not the crate
    let examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples");
    let valid_examples = collect_fil_files(examples_path);

    assert!(
        !valid_examples.is_empty(),
        "No valid examples found in examples/"
    );

    let mut failed_examples = Vec::new();

    for example_path in &valid_examples {
        let output_filename = format!(
            "{}.svg",
            example_path.file_stem().unwrap().to_string_lossy()
        );
        let output_path = temp_dir.path().join(output_filename);

        let args = Args {
            input: example_path.to_string_lossy().to_string(),
            output: output_path.to_string_lossy().to_string(),
            config: None,
            log_level: "off".to_string(),
        };

        if let Err(e) = run(&args) {
            failed_examples.push((example_path.clone(), e));
        }
    }

    if !failed_examples.is_empty() {
        eprintln!("\nValid examples that failed:");
        for (path, err) in &failed_examples {
            eprintln!("  - {}: {}", path.display(), err);
        }
        panic!(
            "{} valid example(s) failed unexpectedly",
            failed_examples.len()
        );
    }

    println!("✅ All {} valid examples passed", valid_examples.len());
}

#[test]
fn e2e_smoke_test_error_examples() {
    // Create a temporary directory for test outputs
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Examples are at workspace root, relative to workspace not the crate
    let examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("errors");
    let error_examples = collect_fil_files(examples_path);

    assert!(
        !error_examples.is_empty(),
        "No error examples found in examples/errors/"
    );

    let mut unexpectedly_succeeded = Vec::new();

    for example_path in &error_examples {
        let output_filename = format!(
            "error_{}.svg",
            example_path.file_stem().unwrap().to_string_lossy()
        );
        let output_path = temp_dir.path().join(output_filename);

        let args = Args {
            input: example_path.to_string_lossy().to_string(),
            output: output_path.to_string_lossy().to_string(),
            config: None,
            log_level: "off".to_string(),
        };

        if run(&args).is_ok() {
            unexpectedly_succeeded.push(example_path.clone());
        }
    }

    if !unexpectedly_succeeded.is_empty() {
        eprintln!("\nError examples that unexpectedly succeeded:");
        for path in &unexpectedly_succeeded {
            eprintln!("  - {}", path.display());
        }
        panic!(
            "{} error example(s) succeeded unexpectedly",
            unexpectedly_succeeded.len()
        );
    }

    println!(
        "✅ All {} error examples failed as expected",
        error_examples.len()
    );
}
