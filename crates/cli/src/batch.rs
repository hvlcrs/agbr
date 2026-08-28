//! Batch input expansion and bounded-concurrency helpers (issue #16).
//!
//! Batch commands accept one or more inputs, each of which may be a concrete
//! file, a directory (scanned recursively for supported image files), or a
//! glob pattern. A leading `~/` is expanded to the home directory.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::Value;

/// Image extensions accepted when scanning a directory.
const RAW_EXTENSIONS: &[&str] = &[
    "dng", "arw", "nef", "cr2", "cr3", "raf", "orf", "rw2", "pef", "srw", "rwl", "3fr", "fff",
    "tif", "tiff", "jpg", "jpeg", "png",
];

/// Expand a list of user inputs (files, directories, or glob patterns) into a
/// de-duplicated, sorted list of concrete file paths.
pub fn expand(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for input in inputs {
        let input = expand_tilde(input);
        let s = input.to_string_lossy();

        if input.is_dir() {
            out.extend(scan_dir(&input)?);
        } else if has_glob_meta(&s) {
            let mut matched = false;
            let pattern = s.to_string();
            for entry in glob::glob(&pattern)
                .with_context(|| format!("invalid glob pattern: {}", input.display()))?
            {
                let path = entry?;
                if path.is_file() {
                    out.push(path);
                    matched = true;
                }
            }
            if !matched {
                bail!("no files matched pattern: {}", input.display());
            }
        } else if input.is_file() {
            out.push(input);
        } else {
            bail!("no such file or directory: {}", input.display());
        }
    }

    out.sort();
    out.dedup();
    if out.is_empty() {
        bail!("no input photos found");
    }
    Ok(out)
}

/// Run `f(0..n)` with at most `jobs` concurrent worker threads and return the
/// results in input order. `jobs <= 1` (or a single item) runs sequentially.
pub fn map_concurrent<F>(n: usize, jobs: usize, f: F) -> Vec<Result<Value>>
where
    F: Fn(usize) -> Result<Value> + Sync,
{
    if n == 0 {
        return Vec::new();
    }
    if n == 1 || jobs <= 1 {
        return (0..n).map(f).collect();
    }

    use std::sync::{mpsc, Mutex};

    let jobs = jobs.min(n);
    let (tx, rx) = mpsc::channel::<usize>();
    for i in 0..n {
        let _ = tx.send(i);
    }
    drop(tx);

    let rx = Mutex::new(rx);
    let slots: Vec<Mutex<Option<Result<Value>>>> = (0..n).map(|_| Mutex::new(None)).collect();

    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| loop {
                let next = rx.lock().expect("poisoned").recv();
                let Ok(i) = next else { break };
                let result = f(i);
                *slots[i].lock().expect("poisoned") = Some(result);
            });
        }
    });

    slots
        .into_iter()
        .map(|s| s.into_inner().expect("poisoned").expect("job ran"))
        .collect()
}

/// Build a batch summary from ordered per-item results and a label accessor.
pub fn summarize(inputs: &[(String, String)], results: Vec<Result<Value>>, jobs: usize) -> Value {
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut items = Vec::with_capacity(results.len());

    for ((source, label), result) in inputs.iter().zip(results) {
        match result {
            Ok(mut value) => {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("source".to_string(), Value::String(source.clone()));
                }
                succeeded += 1;
                items.push(value);
            }
            Err(e) => {
                failed += 1;
                items.push(serde_json::json!({
                    "source": source,
                    "label": label,
                    "status": "error",
                    "error": format!("{e:#}"),
                }));
            }
        }
    }

    serde_json::json!({
        "processed": inputs.len(),
        "succeeded": succeeded,
        "failed": failed,
        "jobs": jobs.max(1),
        "results": items,
    })
}

fn has_glob_meta(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains(']')
}

fn expand_tilde(path: &Path) -> PathBuf {
    if let Some(rest) = path.to_str().and_then(|s| s.strip_prefix("~/")) {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

fn scan_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(scan_dir(&path)?);
        } else if path.is_file() && has_supported_ext(&path) {
            files.push(path);
        }
    }
    Ok(files)
}

fn has_supported_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| RAW_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("agbr-batch-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn expands_files_directories_and_globs() {
        let dir = temp_dir("expand");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.dng"), b"").unwrap();
        std::fs::write(dir.join("sub").join("b.arw"), b"").unwrap();
        std::fs::write(dir.join("sub").join("skip.txt"), b"").unwrap();

        let inputs = vec![dir.clone()];
        let files = expand(&inputs).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&dir.join("a.dng")));
        assert!(files.contains(&dir.join("sub").join("b.arw")));

        let glob_input = vec![dir.join("sub").join("*.arw")];
        let files = expand(&glob_input).unwrap();
        assert_eq!(files, vec![dir.join("sub").join("b.arw")]);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deduplicates_and_sorts() {
        let dir = temp_dir("dedup");
        std::fs::write(dir.join("a.dng"), b"").unwrap();
        std::fs::write(dir.join("b.dng"), b"").unwrap();

        let inputs = vec![dir.join("b.dng"), dir.join("a.dng"), dir.join("b.dng")];
        let files = expand(&inputs).unwrap();
        assert_eq!(files, vec![dir.join("a.dng"), dir.join("b.dng")]);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_missing_input() {
        let dir = temp_dir("missing");
        let inputs = vec![dir.join("nope.dng")];
        assert!(expand(&inputs).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_unmatched_glob() {
        let dir = temp_dir("glob-empty");
        let inputs = vec![dir.join("*.dng")];
        assert!(expand(&inputs).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn map_concurrent_preserves_order() {
        let results = map_concurrent(8, 3, |i| Ok(serde_json::json!({ "i": i })));
        for (i, r) in results.iter().enumerate() {
            assert_eq!(r.as_ref().unwrap()["i"], serde_json::json!(i));
        }
    }

    #[test]
    fn map_concurrent_collects_errors() {
        let results = map_concurrent(4, 2, |i| {
            if i % 2 == 0 {
                anyhow::bail!("boom {i}")
            } else {
                Ok(serde_json::json!({ "ok": i }))
            }
        });
        assert!(results[0].is_err());
        assert!(results[1].is_ok());
        assert!(results[2].is_err());
        assert!(results[3].is_ok());
    }

    #[test]
    fn summarize_counts_success_and_failure() {
        let inputs = vec![
            ("/a.dng".to_string(), "a".to_string()),
            ("/b.dng".to_string(), "b".to_string()),
        ];
        let results = vec![
            Ok(serde_json::json!({ "output": "/out/a.jpg" })),
            Err(anyhow::anyhow!("failed")),
        ];
        let summary = summarize(&inputs, results, 2);
        assert_eq!(summary["processed"], 2);
        assert_eq!(summary["succeeded"], 1);
        assert_eq!(summary["failed"], 1);
        assert_eq!(summary["results"][0]["source"], "/a.dng");
        assert_eq!(summary["results"][1]["status"], "error");
    }
}
