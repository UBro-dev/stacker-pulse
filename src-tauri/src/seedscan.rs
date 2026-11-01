//! SeedScan Disk & Draft Scanner Module
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! Guarantees:
//!   1. Read-only: never writes, moves, or deletes files.
//!   2. Offline: never transmits findings over any network socket.
//!   3. Masked: never stores or returns unmasked secrets.

use crate::detect::{self, Finding};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".svn",
    "venv",
    ".venv",
    "__pycache__",
    "site-packages",
    ".cargo",
    ".rustup",
    "AppData/Local/Temp",
];

const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "svg", "mp3", "mp4", "avi", "mkv", "mov",
    "wav", "flac", "zip", "gz", "xz", "bz2", "7z", "rar", "tar", "exe", "dll", "so", "dylib",
    "bin", "o", "a", "pdf", "woff", "woff2", "ttf", "otf", "pyc", "class", "wasm",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScanHit {
    pub path: String,
    /// Inside a project's test/fixtures/examples folder.
    #[serde(default)]
    pub test_folder: bool,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskScanReport {
    pub scanned_files: usize,
    pub total_findings: usize,
    pub hits: Vec<FileScanHit>,
    /// Folders actually scanned (after `~` expansion).
    pub roots: Vec<String>,
    /// Folders the user asked for that do not exist or are not folders.
    pub missing: Vec<String>,
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .filter(|_| cfg!(windows))
        .or_else(|| std::env::var_os("HOME"))
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Expands `~`, `~/x`, `~\x`, `%USERPROFILE%` and `$HOME` at the start of a
/// path. Neither Rust nor PowerShell does this for us, so before this the UI's
/// default "~/Downloads" pointed at a folder literally named "~" and the scan
/// silently found 0 files.
pub fn expand_home(raw: &str) -> PathBuf {
    let t = raw.trim().trim_matches('"');
    for prefix in ["%USERPROFILE%", "$HOME", "~"] {
        if let Some(rest) = t.strip_prefix(prefix) {
            if rest.is_empty() {
                return home_dir();
            }
            if let Some(rest) = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')) {
                return home_dir().join(rest);
            }
        }
    }
    PathBuf::from(t)
}

pub fn default_scan_paths() -> Vec<PathBuf> {
    let home = home_dir();
    ["Downloads", "Documents", "Desktop", "OneDrive", "Dropbox"]
        .iter()
        .map(|d| home.join(d))
        .filter(|p| p.is_dir())
        .collect()
}

fn should_skip_dir(path: &Path) -> bool {
    let s = path.to_string_lossy().replace('\\', "/");
    SKIP_DIRS
        .iter()
        .any(|d| s.contains(&format!("/{d}")) || s.ends_with(d))
}

fn should_skip_file(path: &Path, max_bytes: u64) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if SKIP_EXTS.contains(&ext.to_lowercase().as_str()) {
            return true;
        }
    }
    match fs::metadata(path) {
        Ok(m) => m.len() > max_bytes || m.len() == 0,
        Err(_) => true,
    }
}

fn read_text(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut head = [0u8; 4096];
    let n = file.read(&mut head).ok()?;
    if head[..n].contains(&0) {
        return None;
    }
    let mut rest = Vec::new();
    file.read_to_end(&mut rest).ok()?;
    let mut all = head[..n].to_vec();
    all.extend_from_slice(&rest);
    Some(String::from_utf8_lossy(&all).into_owned())
}

/// Text mode: a draft or a pasted log about to be published.
pub fn scan_text_content(content: &str) -> Vec<Finding> {
    detect::scan_content(content, detect::Mode::Text, None)
}

/// Folders where projects keep test data; keys there are almost always
/// throwaway fixtures (e.g. spl token-lending `tests/fixtures/*.json`).
fn in_test_folder(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy().to_lowercase();
        matches!(
            s.as_str(),
            "test"
                | "tests"
                | "fixtures"
                | "testdata"
                | "test-data"
                | "__tests__"
                | "examples"
                | "spec"
        )
    })
}

pub fn scan_folders(paths: Vec<PathBuf>, max_size_mb: u64, include_hidden: bool) -> DiskScanReport {
    let requested = if paths.is_empty() {
        default_scan_paths()
    } else {
        paths
            .iter()
            .map(|p| expand_home(&p.to_string_lossy()))
            .collect()
    };
    let (target_paths, missing): (Vec<PathBuf>, Vec<PathBuf>) =
        requested.into_iter().partition(|p| p.is_dir());
    let max_bytes = max_size_mb.max(1) * 1024 * 1024;
    let mut scanned_files = 0usize;
    let mut hits = Vec::new();
    let own_user = detect::own_username();

    for root in &target_paths {
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if !include_hidden {
                    let hidden = e
                        .file_name()
                        .to_str()
                        .map(|s| s.starts_with('.') && s != ".")
                        .unwrap_or(false);
                    if hidden {
                        return false;
                    }
                }
                !(e.file_type().is_dir() && should_skip_dir(e.path()))
            });

        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if should_skip_file(path, max_bytes) {
                continue;
            }
            scanned_files += 1;
            if let Some(content) = read_text(path) {
                let found = detect::scan_content(&content, detect::Mode::Disk, own_user.as_deref());
                if !found.is_empty() {
                    hits.push(FileScanHit {
                        path: path.display().to_string(),
                        test_folder: in_test_folder(path),
                        findings: found,
                    });
                }
            }
        }
    }

    let total_findings = hits.iter().map(|h| h.findings.len()).sum();
    DiskScanReport {
        scanned_files,
        total_findings,
        hits,
        roots: target_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        missing: missing.iter().map(|p| p.display().to_string()).collect(),
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn tilde_and_profile_variables_expand_to_home() {
        let home = home_dir();
        assert_eq!(expand_home("~"), home);
        assert_eq!(expand_home("~/Downloads"), home.join("Downloads"));
        assert_eq!(expand_home("~\\Documents"), home.join("Documents"));
        assert_eq!(expand_home("%USERPROFILE%\\Desktop"), home.join("Desktop"));
        assert_eq!(expand_home(" \"$HOME/x\" "), home.join("x"));
        // Not a home prefix: left alone.
        assert_eq!(expand_home("~user/x"), PathBuf::from("~user/x"));
        assert_eq!(expand_home("D:\\Backups"), PathBuf::from("D:\\Backups"));
    }

    #[test]
    fn scan_reports_missing_folders_and_finds_leaks_under_tilde() {
        let dir = std::env::temp_dir().join(format!("sp_seedscan_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("notes.txt"),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n").unwrap();
        let missing = "~/definitely-not-a-folder-sp".to_string();
        let rep = scan_folders(vec![dir.clone(), PathBuf::from(&missing)], 8, false);
        assert_eq!(rep.scanned_files, 1);
        assert_eq!(rep.total_findings, 1);
        assert_eq!(rep.roots, vec![dir.display().to_string()]);
        assert_eq!(
            rep.missing,
            vec![home_dir()
                .join("definitely-not-a-folder-sp")
                .display()
                .to_string()]
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
