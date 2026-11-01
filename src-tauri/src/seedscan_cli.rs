//! Author: UBro-dev | License: MIT (Copyright (c) 2026 UBro-dev)
//! seedscan — find crypto secrets left in plain text on your own disk.
//!
//! Design rules, in order of importance:
//!   1. It never opens a network socket. There is no telemetry and no update
//!      check. You can verify that: nothing here imports a network crate.
//!   2. It never writes, moves or deletes anything. Read-only, always.
//!   3. It never prints a secret in full. Findings are masked.

mod detect;

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use walkdir::WalkDir;

use detect::{Finding, Kind};

#[derive(Parser)]
#[command(
    name = "seedscan",
    version,
    about = "Find seed phrases and private keys left in plain text on your own disk",
    long_about = "seedscan walks folders you name and reports files that contain \
                  crypto secrets in plain text: BIP39 seed phrases, extended \
                  private keys, WIF keys, Nostr nsec keys and Solana keypair \
                  files.\n\nIt is fully offline, read-only, and masks every \
                  finding. Checksums are validated, so ordinary text does not \
                  trigger it."
)]
struct Args {
    /// Folders (or files) to scan. Defaults to common places secrets end up.
    paths: Vec<PathBuf>,

    /// Skip files larger than this, in megabytes.
    #[arg(long, default_value_t = 8)]
    max_size_mb: u64,

    /// Also scan hidden files and folders.
    #[arg(long)]
    hidden: bool,

    /// Print every file as it is scanned.
    #[arg(short, long)]
    verbose: bool,
}

/// Directories that are pure noise: huge, and secrets there are not the user's
/// own leftovers.
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

/// Extensions that cannot usefully hold a plain-text secret.
const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "svg", "mp3", "mp4", "avi", "mkv", "mov",
    "wav", "flac", "zip", "gz", "xz", "bz2", "7z", "rar", "tar", "exe", "dll", "so", "dylib",
    "bin", "o", "a", "pdf", "woff", "woff2", "ttf", "otf", "pyc", "class", "wasm",
];

fn default_paths() -> Vec<PathBuf> {
    let home = dirs_home();
    ["Downloads", "Documents", "Desktop", "OneDrive", "Dropbox"]
        .iter()
        .map(|d| home.join(d))
        .filter(|p| p.is_dir())
        .collect()
}

/// `~`, `~/x`, `%USERPROFILE%\\x`, `$HOME/x` → under the home folder.
fn expand_home(raw: &str) -> PathBuf {
    let t = raw.trim().trim_matches('"');
    for prefix in ["%USERPROFILE%", "$HOME", "~"] {
        if let Some(rest) = t.strip_prefix(prefix) {
            if rest.is_empty() {
                return dirs_home();
            }
            if let Some(rest) = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')) {
                return dirs_home().join(rest);
            }
        }
    }
    PathBuf::from(t)
}

/// Minimal home-directory lookup; avoids pulling in another dependency.
fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
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

/// Read a file only if it looks like text. Binary files are skipped by
/// checking for NUL bytes in the first chunk.
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

fn scan_file(path: &Path) -> Vec<Finding> {
    let Some(content) = read_text(path) else {
        return Vec::new();
    };
    detect::scan_content(
        &content,
        detect::Mode::Disk,
        detect::own_username().as_deref(),
    )
}

fn main() {
    let args = Args::parse();

    let paths = if args.paths.is_empty() {
        let d = default_paths();
        if d.is_empty() {
            eprintln!("No default folders found. Pass folders to scan, e.g.:");
            eprintln!("  seedscan ~/Documents ~/Downloads");
            std::process::exit(2);
        }
        d
    } else {
        // PowerShell does not expand ~ for programs, so do it here.
        args.paths
            .iter()
            .map(|p| expand_home(&p.to_string_lossy()))
            .collect()
    };
    for p in &paths {
        if !p.is_dir() {
            eprintln!("warning: not a folder, skipped: {}", p.display());
        }
    }

    let max_bytes = args.max_size_mb * 1024 * 1024;
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let _ = writeln!(
        out,
        "seedscan {} — offline, read-only. Nothing leaves this machine.\n",
        env!("CARGO_PKG_VERSION")
    );
    for p in &paths {
        let _ = writeln!(out, "  scanning {}", p.display());
    }
    let _ = writeln!(out);

    let mut scanned = 0usize;
    let mut hits: Vec<(PathBuf, Vec<Finding>)> = Vec::new();

    for root in &paths {
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if !args.hidden {
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
            if args.verbose {
                let _ = writeln!(out, "    {}", path.display());
            }
            scanned += 1;
            let found = scan_file(path);
            if !found.is_empty() {
                hits.push((path.to_path_buf(), found));
            }
        }
    }

    let total: usize = hits.iter().map(|(_, f)| f.len()).sum();

    if hits.is_empty() {
        let _ = writeln!(out, "Scanned {scanned} files. No plain-text secrets found.");
        let _ = writeln!(
            out,
            "\nThat is not a guarantee: seedscan only knows the formats it \
             checks, and only looks where you pointed it."
        );
        return;
    }

    let _ = writeln!(
        out,
        "Scanned {scanned} files. Found {total} item(s) in {} file(s).\n",
        hits.len()
    );
    let _ = writeln!(out, "{}", "=".repeat(68));

    for (path, findings) in &hits {
        let _ = writeln!(out, "\n{}", path.display());
        for f in findings {
            if f.line > 0 {
                let _ = writeln!(out, "  line {:<5} {}", f.line, f);
            } else {
                let _ = writeln!(out, "  whole file  {f}");
            }
        }
        // One piece of advice per distinct kind in this file.
        let mut kinds: Vec<Kind> = findings.iter().map(|f| f.kind).collect();
        kinds.dedup();
        for k in kinds {
            let _ = writeln!(out, "    -> {}", k.advice());
        }
    }

    let _ = writeln!(out, "\n{}", "=".repeat(68));
    let _ = writeln!(
        out,
        "\nSecrets are shown masked. seedscan did not copy, move or send anything.\n\
         If any of these files were ever in a synced folder, a backup, a chat or \n\
         a git repository, assume the secret is compromised and move the funds."
    );
}
