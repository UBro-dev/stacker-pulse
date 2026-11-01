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

/// True if a directory found *during the walk* is one we never look inside.
/// Matches whole path components at the end of the path, case-insensitively,
/// so "target" skips `…/target` but not `…/targeted_notes` or `…/mytarget`,
/// and "AppData/Local/Temp" needs all three components in that order.
/// Never call this for a folder the user chose: see `filter_entry` callers.
fn should_skip_dir(path: &Path) -> bool {
    let comps: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect();
    SKIP_DIRS.iter().any(|d| {
        let parts: Vec<String> = d.split('/').map(|p| p.to_lowercase()).collect();
        comps.len() >= parts.len() && comps[comps.len() - parts.len()..] == parts[..]
    })
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
                // A folder the user chose explicitly is always scanned, even if
                // it is hidden (~/.config/solana) or under AppData/Local/Temp.
                if e.depth() == 0 {
                    return true;
                }
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

    const SEED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n";

    fn fresh(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sp_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// The Windows CI failure: a chosen folder under AppData\Local\Temp was
    /// skipped as a whole, and the scan reported "0 files" as if clean.
    #[test]
    fn a_chosen_folder_inside_appdata_temp_is_scanned() {
        let base = fresh("temp_root");
        let proj = base.join("AppData").join("Local").join("Temp").join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("notes.txt"), SEED).unwrap();
        let rep = scan_folders(vec![proj], 8, false);
        assert_eq!(rep.scanned_files, 1);
        let _ = fs::remove_dir_all(&base);
    }

    /// Scanning a whole profile still skips the Temp folder found inside it.
    #[test]
    fn appdata_temp_is_still_skipped_when_found_during_the_walk() {
        let home = fresh("profile");
        let junk = home.join("AppData").join("Local").join("Temp").join("junk");
        fs::create_dir_all(&junk).unwrap();
        fs::write(junk.join("seed.txt"), SEED).unwrap();
        fs::create_dir_all(home.join("Documents")).unwrap();
        fs::write(home.join("Documents").join("seed.txt"), SEED).unwrap();
        let rep = scan_folders(vec![home.clone()], 8, false);
        assert_eq!(rep.scanned_files, 1, "only Documents/seed.txt");
        assert!(rep.hits[0].path.contains("Documents"));
        let _ = fs::remove_dir_all(&home);
    }

    /// A hidden folder the user names explicitly is scanned without --hidden:
    /// wallets keep their files in ~/.electrum, ~/.config/solana and the like.
    #[test]
    fn a_chosen_hidden_folder_is_scanned_without_the_hidden_flag() {
        let base = fresh("hidden_root");
        // The chosen folder's own name starts with a dot, like ~/.electrum.
        let wallet = base.join(".electrum");
        fs::create_dir_all(&wallet).unwrap();
        fs::write(wallet.join("backup.txt"), SEED).unwrap();
        let rep = scan_folders(vec![wallet], 8, false);
        assert_eq!(rep.scanned_files, 1);
        // ...but hidden folders met during the walk are still skipped.
        let rep = scan_folders(vec![base.clone()], 8, false);
        assert_eq!(rep.scanned_files, 0);
        let _ = fs::remove_dir_all(&base);
    }

    /// Skip rules match whole folder names, not substrings of the path.
    #[test]
    fn skip_rules_match_whole_folder_names() {
        let base = fresh("names");
        for d in ["target", "targeted_notes", "mytarget", "Target"] {
            fs::create_dir_all(base.join(d)).unwrap();
            fs::write(base.join(d).join("s.txt"), SEED).unwrap();
        }
        let rep = scan_folders(vec![base.clone()], 8, false);
        let mut seen: Vec<String> = rep
            .hits
            .iter()
            .map(|h| {
                Path::new(&h.path)
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        seen.sort();
        assert_eq!(seen, vec!["mytarget", "targeted_notes"]);
        let _ = fs::remove_dir_all(&base);
    }
}
