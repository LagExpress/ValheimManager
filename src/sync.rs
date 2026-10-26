use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use glob::{MatchOptions, Pattern};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::sftp::{RemoteClient, WrapperReport, WrapperStatus, REMOTE_ROOT};

pub const STATE_FILE_NAME: &str = ".sync_state.json";
pub const IGNORE_FILE_NAME: &str = ".sync_ignore";

pub const DEFAULT_IGNORED_PATTERNS: &[&str] = &[
    "config/PUPSafeGraustenCombiner",
    "config/PUPSafeGraustenCombiner/*",
    "config/PlanBuild/blueprints",
    "config/PlanBuild/blueprints/*",
    "config/PlanBuild/blueprints/Cuboide_Torre.blueprint",
    "config/PauseMyServer.state.json",
    "*.state.json",
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub staging_dir: String,
    pub auto_refresh_staging: bool,
    pub language: Option<crate::i18n::Language>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: String::new(),
            password: String::new(),
            staging_dir: default_staging_dir().display().to_string(),
            auto_refresh_staging: true,
            language: None,
        }
    }
}

impl SyncConfig {
    pub fn path() -> PathBuf {
        config_dir().join("sync.json")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("Could not read {}: {}", path.display(), e))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("Could not parse {}: {}", path.display(), e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        let dir = path
            .parent()
            .ok_or_else(|| "Invalid sync config path".to_string())?;
        fs::create_dir_all(dir)
            .map_err(|e| format!("Could not create {}: {}", dir.display(), e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
        }

        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Could not serialize sync config: {}", e))?;

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)
                .map_err(|e| format!("Could not write {}: {}", path.display(), e))?;
            file.write_all(text.as_bytes())
                .map_err(|e| format!("Could not write {}: {}", path.display(), e))?;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }

        #[cfg(not(unix))]
        {
            fs::write(&path, text)
                .map_err(|e| format!("Could not write {}: {}", path.display(), e))?;
        }

        Ok(())
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("valheim-modpack-studio")
}

pub fn default_staging_dir() -> PathBuf {
    dirs::desktop_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Desktop")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("valheim")
}

pub fn expand_tilde(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    }
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(trimmed)
}

pub fn paths_overlap(a: &Path, b: &Path) -> bool {
    let a = comparable_path(a);
    let b = comparable_path(b);
    a == b || a.starts_with(&b) || b.starts_with(&a)
}

fn comparable_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }

    if let Some(parent) = path.parent() {
        if let Ok(parent) = fs::canonicalize(parent) {
            if let Some(name) = path.file_name() {
                return parent.join(name);
            }
        }
    }

    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

pub fn parse_ignore_text(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

pub fn effective_patterns(custom: &[String]) -> Vec<String> {
    let mut patterns: Vec<String> = DEFAULT_IGNORED_PATTERNS
        .iter()
        .map(|pattern| pattern.to_string())
        .collect();
    patterns.extend(custom.iter().cloned());
    patterns
}

fn normalize_rel(path: &str) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}

pub fn is_ignored(rel_path: &str, patterns: &[String]) -> bool {
    let normalized = normalize_rel(rel_path);
    if normalized.is_empty() {
        return false;
    }

    let base = normalized.rsplit('/').next().unwrap_or(&normalized);
    let options = MatchOptions {
        case_sensitive: !cfg!(windows),
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    for pattern in patterns {
        let pattern = normalize_rel(pattern);
        if pattern.is_empty() {
            continue;
        }

        if normalized == pattern
            || normalized
                .strip_prefix(&pattern)
                .is_some_and(|rest| rest.starts_with('/'))
        {
            return true;
        }

        if let Ok(compiled) = Pattern::new(&pattern) {
            if compiled.matches_with(&normalized, options) || compiled.matches_with(base, options) {
                return true;
            }
        }
    }

    false
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Could not open {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Could not read {}: {}", path.display(), e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect())
}

pub fn scan_local_files(
    staging: &Path,
    patterns: &[String],
) -> Result<BTreeMap<String, String>, String> {
    if staging.as_os_str().is_empty() {
        return Err("Staging folder cannot be empty.".to_string());
    }
    if !staging.exists() {
        return Err(format!(
            "Staging folder does not exist: {}",
            staging.display()
        ));
    }

    let mut files = BTreeMap::new();
    for sub in ["config", "plugins"] {
        let base = staging.join(sub);
        if !base.exists() {
            continue;
        }

        for entry in WalkDir::new(&base)
            .into_iter()
            .filter_entry(|entry| !is_ignored(&rel_from(entry.path(), &base, sub), patterns))
        {
            let entry = entry.map_err(|e| format!("Could not scan {}: {}", base.display(), e))?;
            if !entry.path().is_file() {
                continue;
            }

            let rel = entry
                .path()
                .strip_prefix(staging)
                .map_err(|e| format!("Could not compute relative path: {}", e))?;
            let rel = rel.to_string_lossy().replace('\\', "/");

            if is_ignored(&rel, patterns) {
                continue;
            }

            files.insert(rel, hash_file(entry.path())?);
        }
    }

    Ok(files)
}

pub fn state_path(staging: &Path) -> PathBuf {
    staging.join(STATE_FILE_NAME)
}

pub fn load_state(path: &Path) -> Result<BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Could not read {}: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("Could not parse {}: {}", path.display(), e))
}

pub fn save_state(path: &Path, state: &BTreeMap<String, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {}", parent.display(), e))?;
    }
    let text = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Could not serialize sync state: {}", e))?;
    fs::write(path, text).map_err(|e| format!("Could not write {}: {}", path.display(), e))
}

#[derive(Debug, Clone, Default)]
pub struct StagingStats {
    pub copied: usize,
    pub pruned: usize,
    pub missing_sources: Vec<String>,
}

pub fn refresh_staging(
    source_bepinex: &Path,
    staging: &Path,
    patterns: &[String],
) -> Result<StagingStats, String> {
    if staging.as_os_str().is_empty() {
        return Err("Staging folder cannot be empty.".to_string());
    }
    if !source_bepinex.is_dir() {
        return Err(format!(
            "Source BepInEx folder not found: {}",
            source_bepinex.display()
        ));
    }
    if paths_overlap(source_bepinex, staging) {
        return Err(format!(
            "Staging folder must not overlap the source BepInEx folder: {}",
            staging.display()
        ));
    }

    let mut stats = StagingStats::default();
    fs::create_dir_all(staging)
        .map_err(|e| format!("Could not create {}: {}", staging.display(), e))?;

    for sub in ["config", "plugins"] {
        let source = source_bepinex.join(sub);
        let destination = staging.join(sub);

        if !source.is_dir() {
            stats.missing_sources.push(sub.to_string());
            continue;
        }

        fs::create_dir_all(&destination)
            .map_err(|e| format!("Could not create {}: {}", destination.display(), e))?;
        copy_tree(&source, &destination, sub, patterns, &mut stats)?;
        prune_tree(&source, &destination, sub, patterns, &mut stats)?;
    }

    Ok(stats)
}

fn rel_from(path: &Path, base: &Path, prefix: &str) -> String {
    let rel = path
        .strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if rel.is_empty() {
        prefix.to_string()
    } else {
        format!("{}/{}", prefix, rel)
    }
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    prefix: &str,
    patterns: &[String],
    stats: &mut StagingStats,
) -> Result<(), String> {
    for entry in WalkDir::new(source)
        .into_iter()
        .filter_entry(|entry| !is_ignored(&rel_from(entry.path(), source, prefix), patterns))
    {
        let entry = entry.map_err(|e| format!("Could not read {}: {}", source.display(), e))?;
        if entry.path() == source {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(source)
            .map_err(|e| format!("Could not compute relative path: {}", e))?;
        let target = destination.join(rel);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .map_err(|e| format!("Could not create {}: {}", target.display(), e))?;
        } else if entry.path().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Could not create {}: {}", parent.display(), e))?;
            }
            fs::copy(entry.path(), &target).map_err(|e| {
                format!(
                    "Could not copy {} to {}: {}",
                    entry.path().display(),
                    target.display(),
                    e
                )
            })?;
            stats.copied += 1;
        }
    }

    Ok(())
}

fn prune_tree(
    source: &Path,
    destination: &Path,
    prefix: &str,
    patterns: &[String],
    stats: &mut StagingStats,
) -> Result<(), String> {
    for entry in WalkDir::new(destination)
        .contents_first(true)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let rel = rel_from(entry.path(), destination, prefix);
        if is_ignored(&rel, patterns) {
            continue;
        }

        if entry.file_type().is_dir() {
            if entry.path() == destination {
                continue;
            }
            let empty = fs::read_dir(entry.path())
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false);
            if empty {
                let _ = fs::remove_dir(entry.path());
            }
        } else if entry.path().is_file() {
            let rel_to_source = entry
                .path()
                .strip_prefix(destination)
                .map_err(|e| format!("Could not compute relative path: {}", e))?;
            if !source.join(rel_to_source).exists() {
                fs::remove_file(entry.path())
                    .map_err(|e| format!("Could not remove {}: {}", entry.path().display(), e))?;
                stats.pruned += 1;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Preview,
    SyncChanges,
    ForceAll,
    UploadOnly,
    CheckWrapper,
    RefreshStaging,
    TestConnection,
}

impl SyncMode {
    pub fn label(&self) -> &'static str {
        match self {
            SyncMode::Preview => "preview",
            SyncMode::SyncChanges => "sync changes",
            SyncMode::ForceAll => "force upload all",
            SyncMode::UploadOnly => "upload only",
            SyncMode::CheckWrapper => "check wrapper",
            SyncMode::RefreshStaging => "refresh staging",
            SyncMode::TestConnection => "test connection",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct SyncLog {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub current: usize,
    pub total: usize,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct PreviewResult {
    pub uploads: Vec<String>,
    pub deletions: Vec<String>,
    pub local_count: usize,
    pub remote_count: usize,
    pub wrapper: WrapperReport,
}

#[derive(Debug, Clone, Default)]
pub struct SyncSummary {
    pub uploaded: usize,
    pub deleted: usize,
    pub failed_uploads: usize,
    pub failed_deletes: usize,
    pub local_count: usize,
    pub staged_copied: usize,
    pub staged_pruned: usize,
    pub wrapper: Option<WrapperReport>,
    pub elapsed_secs: f32,
    pub cancelled: bool,
}

#[derive(Debug)]
pub enum SyncEvent {
    Log(SyncLog),
    Progress(Progress),
    Preview(PreviewResult),
    Finished(Result<SyncSummary, String>),
}

pub type EventSink = Box<dyn Fn(SyncEvent) + Send + 'static>;

pub struct SyncRequest {
    pub mode: SyncMode,
    pub config: SyncConfig,
    pub source_bepinex: Option<PathBuf>,
    pub ignore_patterns: Vec<String>,
    pub refresh_staging: bool,
    pub cancel: Arc<AtomicBool>,
}

pub fn spawn(request: SyncRequest, sink: EventSink) -> JoinHandle<()> {
    thread::spawn(move || {
        let result = run_sync(&request, &sink);
        sink(SyncEvent::Finished(result));
    })
}

fn log(sink: &EventSink, level: LogLevel, message: impl Into<String>) {
    sink(SyncEvent::Log(SyncLog {
        level,
        message: message.into(),
    }));
}

fn progress(sink: &EventSink, current: usize, total: usize, label: impl Into<String>) {
    sink(SyncEvent::Progress(Progress {
        current,
        total,
        label: label.into(),
    }));
}

fn run_sync(request: &SyncRequest, sink: &EventSink) -> Result<SyncSummary, String> {
    let started = Instant::now();
    let patterns = effective_patterns(&request.ignore_patterns);
    let staging = expand_tilde(&request.config.staging_dir);
    if staging.as_os_str().is_empty() {
        return Err("Staging folder cannot be empty.".to_string());
    }
    let mut summary = SyncSummary::default();

    log(sink, LogLevel::Info, "=== Valheim Server Sync ===");

    if request.mode == SyncMode::RefreshStaging || request.refresh_staging {
        let source = request
            .source_bepinex
            .as_ref()
            .ok_or_else(|| "No source profile selected".to_string())?;
        if !source.is_dir() {
            return Err(format!(
                "Source BepInEx folder not found: {}",
                source.display()
            ));
        }
        if paths_overlap(source, &staging) {
            return Err(format!(
                "Staging folder must not overlap the source BepInEx folder: {}",
                staging.display()
            ));
        }

        log(
            sink,
            LogLevel::Info,
            format!("Refreshing staging from {}...", source.display()),
        );
        let stats = refresh_staging(source, &staging, &patterns)?;
        summary.staged_copied = stats.copied;
        summary.staged_pruned = stats.pruned;

        for sub in stats.missing_sources {
            log(
                sink,
                LogLevel::Warning,
                format!("No {} folder found in the source profile.", sub),
            );
        }
        log(
            sink,
            LogLevel::Success,
            format!(
                "Staging updated: {} copied, {} pruned.",
                stats.copied, stats.pruned
            ),
        );
    }

    if request.mode == SyncMode::RefreshStaging {
        summary.elapsed_secs = started.elapsed().as_secs_f32();
        return Ok(summary);
    }

    let needs_staging = matches!(
        request.mode,
        SyncMode::Preview | SyncMode::SyncChanges | SyncMode::ForceAll | SyncMode::UploadOnly
    );

    if needs_staging && !staging.exists() {
        return Err(format!(
            "Staging folder does not exist: {}",
            staging.display()
        ));
    }

    let current_hashes = if needs_staging {
        log(sink, LogLevel::Info, "Scanning local files...");
        let hashes = scan_local_files(&staging, &patterns)?;
        summary.local_count = hashes.len();
        log(
            sink,
            LogLevel::Info,
            format!("Found {} local file(s) to consider.", hashes.len()),
        );
        hashes
    } else {
        BTreeMap::new()
    };

    log(
        sink,
        LogLevel::Info,
        format!(
            "Connecting to {}:{} as {}...",
            request.config.host, request.config.port, request.config.username
        ),
    );
    let (client, fingerprint) = RemoteClient::connect(
        &request.config.host,
        request.config.port,
        &request.config.username,
        &request.config.password,
    )?;

    if let Some(fingerprint) = fingerprint {
        log(
            sink,
            LogLevel::Info,
            format!("Host key SHA256: {}", fingerprint),
        );
    }
    log(sink, LogLevel::Success, "Connected.");

    if request.mode == SyncMode::TestConnection {
        match client.list_dir_names(REMOTE_ROOT) {
            Ok(entries) => log(
                sink,
                LogLevel::Success,
                format!(
                    "Remote {} folder is readable ({} entries).",
                    REMOTE_ROOT,
                    entries.len()
                ),
            ),
            Err(e) => {
                return Err(format!(
                    "Connected, but could not read {}: {}",
                    REMOTE_ROOT, e
                ))
            }
        }
        summary.elapsed_secs = started.elapsed().as_secs_f32();
        return Ok(summary);
    }

    if request.mode == SyncMode::CheckWrapper {
        log(
            sink,
            LogLevel::Info,
            "Checking server binary and wrapper...",
        );
        let report = client.check_and_fix_wrapper()?;
        log_wrapper_report(sink, &report);
        summary.wrapper = Some(report);
        summary.elapsed_secs = started.elapsed().as_secs_f32();
        return Ok(summary);
    }

    let force_all = request.mode == SyncMode::ForceAll;
    let delete_removed = matches!(
        request.mode,
        SyncMode::Preview | SyncMode::SyncChanges | SyncMode::ForceAll
    );

    let last_state = if force_all {
        BTreeMap::new()
    } else {
        match load_state(&state_path(&staging)) {
            Ok(state) => state,
            Err(e) => {
                log(
                    sink,
                    LogLevel::Warning,
                    format!("Could not load sync state, starting fresh: {}", e),
                );
                BTreeMap::new()
            }
        }
    };

    let mut files_to_upload: Vec<String> = current_hashes
        .iter()
        .filter(|(rel, hash)| force_all || last_state.get(*rel) != Some(*hash))
        .map(|(rel, _)| rel.clone())
        .collect();

    let mut files_to_delete = Vec::new();
    let mut remote_count = 0;

    if delete_removed {
        if current_hashes.is_empty() {
            log(
                sink,
                LogLevel::Warning,
                "Local scan found zero files; refusing to delete remote files.",
            );
        } else {
            log(
                sink,
                LogLevel::Info,
                "Scanning remote server for removed files...",
            );
            let remote_config =
                client.list_files(&format!("{}/config", REMOTE_ROOT), "config", &patterns)?;
            let remote_plugins =
                client.list_files(&format!("{}/plugins", REMOTE_ROOT), "plugins", &patterns)?;

            let mut remote_files: HashSet<String> = HashSet::new();
            remote_files.extend(remote_config);
            remote_files.extend(remote_plugins);
            remote_count = remote_files.len();

            files_to_delete = remote_files
                .iter()
                .filter(|file| {
                    !current_hashes.contains_key(*file) && !is_ignored(file.as_str(), &patterns)
                })
                .cloned()
                .collect();
            files_to_delete.sort();

            for rel in current_hashes.keys() {
                if !remote_files.contains(rel) {
                    files_to_upload.push(rel.clone());
                }
            }

            log(
                sink,
                LogLevel::Info,
                format!("Found {} remote file(s).", remote_count),
            );
        }
    }

    files_to_upload.sort();
    files_to_upload.dedup();

    if request.mode == SyncMode::Preview {
        log(sink, LogLevel::Info, "Inspecting wrapper status...");
        let wrapper = match client.inspect_wrapper() {
            Ok(report) => report,
            Err(e) => {
                log(
                    sink,
                    LogLevel::Warning,
                    format!("Could not inspect wrapper: {}", e),
                );
                WrapperReport {
                    status: WrapperStatus::Unknown,
                    warnings: vec![e],
                    ..Default::default()
                }
            }
        };
        log_wrapper_report(sink, &wrapper);

        sink(SyncEvent::Preview(PreviewResult {
            uploads: files_to_upload.clone(),
            deletions: files_to_delete.clone(),
            local_count: current_hashes.len(),
            remote_count,
            wrapper,
        }));

        summary.uploaded = files_to_upload.len();
        summary.deleted = files_to_delete.len();
        summary.elapsed_secs = started.elapsed().as_secs_f32();
        return Ok(summary);
    }

    log(
        sink,
        LogLevel::Info,
        "Checking server binary and wrapper...",
    );
    match client.check_and_fix_wrapper() {
        Ok(report) => {
            log_wrapper_report(sink, &report);
            summary.wrapper = Some(report);
        }
        Err(e) => log(
            sink,
            LogLevel::Warning,
            format!("Wrapper check failed (continuing): {}", e),
        ),
    }

    if files_to_upload.is_empty() && files_to_delete.is_empty() {
        log(
            sink,
            LogLevel::Success,
            format!(
                "All {} files are already up to date with the server.",
                current_hashes.len()
            ),
        );
        summary.elapsed_secs = started.elapsed().as_secs_f32();
        return Ok(summary);
    }

    let total_operations = files_to_delete.len() + files_to_upload.len();
    let mut completed = 0;

    if !files_to_delete.is_empty() {
        log(
            sink,
            LogLevel::Info,
            format!(
                "Deleting {} removed file(s) from the server...",
                files_to_delete.len()
            ),
        );

        for rel in &files_to_delete {
            if request.cancel.load(Ordering::Relaxed) {
                summary.cancelled = true;
                break;
            }
            if !is_safe_remote_rel(rel) || is_ignored(rel, &patterns) {
                continue;
            }

            let remote = format!("{}/{}", REMOTE_ROOT, rel);
            match client.remove_file(&remote) {
                Ok(()) => {
                    summary.deleted += 1;
                    let _ = client.clean_empty_dirs(parent_remote(&remote), &patterns);
                    log(sink, LogLevel::Info, format!("Deleted {}", rel));
                }
                Err(e) => {
                    summary.failed_deletes += 1;
                    log(
                        sink,
                        LogLevel::Warning,
                        format!("Could not delete {}: {}", rel, e),
                    );
                }
            }

            completed += 1;
            progress(
                sink,
                completed,
                total_operations,
                format!("Deleted {}", rel),
            );
        }
    }

    let mut new_state = current_hashes.clone();

    if !files_to_upload.is_empty() && !summary.cancelled {
        log(
            sink,
            LogLevel::Info,
            format!("Uploading {} file(s)...", files_to_upload.len()),
        );

        for rel in &files_to_upload {
            if request.cancel.load(Ordering::Relaxed) {
                summary.cancelled = true;
                break;
            }

            let local = staging.join(rel);
            let remote = format!("{}/{}", REMOTE_ROOT, rel);
            let parent = parent_remote(&remote);

            if let Err(e) = client.ensure_dir(parent) {
                summary.failed_uploads += 1;
                new_state.remove(rel);
                log(
                    sink,
                    LogLevel::Error,
                    format!("Could not create remote folder for {}: {}", rel, e),
                );
            } else {
                match client.upload(&local, &remote) {
                    Ok(()) => {
                        summary.uploaded += 1;
                        log(sink, LogLevel::Info, format!("Uploaded {}", rel));
                    }
                    Err(e) => {
                        summary.failed_uploads += 1;
                        new_state.remove(rel);
                        log(
                            sink,
                            LogLevel::Error,
                            format!("Failed to upload {}: {}", rel, e),
                        );
                    }
                }
            }

            completed += 1;
            progress(
                sink,
                completed,
                total_operations,
                format!("Uploaded {}", rel),
            );
        }
    }

    if summary.cancelled {
        log(
            sink,
            LogLevel::Warning,
            "Sync cancelled. Sync state was not updated.",
        );
    } else {
        match save_state(&state_path(&staging), &new_state) {
            Ok(()) => log(sink, LogLevel::Success, "Sync state saved."),
            Err(e) => log(
                sink,
                LogLevel::Warning,
                format!("Could not save sync state: {}", e),
            ),
        }

        if summary.failed_uploads == 0 && summary.failed_deletes == 0 {
            log(sink, LogLevel::Success, "Sync finished successfully.");
        } else {
            log(
                sink,
                LogLevel::Warning,
                format!(
                    "Sync finished with {} upload failure(s) and {} delete failure(s).",
                    summary.failed_uploads, summary.failed_deletes
                ),
            );
        }
    }

    summary.elapsed_secs = started.elapsed().as_secs_f32();
    Ok(summary)
}

fn parent_remote(remote: &str) -> &str {
    remote
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or(remote)
}

fn is_safe_remote_rel(rel: &str) -> bool {
    (rel.starts_with("config/") || rel.starts_with("plugins/"))
        && !rel.split('/').any(|part| part == "..")
}

fn log_wrapper_report(sink: &EventSink, report: &WrapperReport) {
    match report.status {
        WrapperStatus::Intact => log(sink, LogLevel::Success, "Server wrapper is intact."),
        WrapperStatus::ElfNeedsRename => log(
            sink,
            LogLevel::Warning,
            "A game update replaced the wrapper with an ELF binary.",
        ),
        WrapperStatus::Missing => log(sink, LogLevel::Warning, "Server wrapper is missing."),
        WrapperStatus::Unknown => log(sink, LogLevel::Warning, "Server wrapper status is unknown."),
    }

    if report.renamed_elf {
        log(
            sink,
            LogLevel::Success,
            "Renamed the game binary to valheim_server.x86_64.bin.",
        );
    }
    if report.installed {
        log(
            sink,
            LogLevel::Success,
            "Installed the modded server wrapper.",
        );
    }
    if report.symlink_created {
        log(sink, LogLevel::Success, "Created the data symlink.");
    }
    if !report.symlink_present && !report.symlink_created {
        log(
            sink,
            LogLevel::Warning,
            "valheim_server.x86_64_Data symlink is missing.",
        );
    }
    for warning in &report.warnings {
        log(sink, LogLevel::Warning, warning.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn default_ignore_patterns_match() {
        let patterns = effective_patterns(&[]);

        assert!(is_ignored("config/PUPSafeGraustenCombiner", &patterns));
        assert!(is_ignored(
            "config/PUPSafeGraustenCombiner/ClusterCache/file.sub",
            &patterns
        ));
        assert!(is_ignored(
            "config/PlanBuild/blueprints/Cuboide_Torre.blueprint",
            &patterns
        ));
        assert!(is_ignored("config/PauseMyServer.state.json", &patterns));
        assert!(is_ignored("config/anything.state.json", &patterns));
        assert!(!is_ignored("config/BepInEx.cfg", &patterns));
        assert!(!is_ignored("plugins/ValheimPlus.dll", &patterns));
    }

    #[test]
    fn custom_ignore_patterns_skip_comments() {
        let custom = parse_ignore_text("# comment\n\nconfig/foo\n  plugins/bar/*  \n");
        assert_eq!(custom, vec!["config/foo", "plugins/bar/*"]);
    }

    #[test]
    fn custom_ignore_patterns_match_files_and_directories() {
        let patterns = vec!["plugins/Example/*".to_string(), "*.secret".to_string()];

        assert!(is_ignored("plugins/Example/Example.dll", &patterns));
        assert!(is_ignored("plugins/Example/nested/file.cfg", &patterns));
        assert!(is_ignored("config/credentials.secret", &patterns));
        assert!(!is_ignored("plugins/Other/Other.dll", &patterns));
        assert!(!is_ignored("config/BepInEx.cfg", &patterns));
    }

    #[test]
    fn staging_and_source_must_not_overlap() {
        let root = tempdir().unwrap();
        let source = root.path().join("profile").join("BepInEx");
        let staging = root.path().join("staging");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&staging).unwrap();

        assert!(paths_overlap(&source, &source));
        assert!(paths_overlap(&source, &source.join("config")));
        assert!(paths_overlap(&source.join("config"), &source));
        assert!(paths_overlap(&source, &root.path().join("profile")));
        assert!(!paths_overlap(&source, &staging));
    }

    #[test]
    fn remote_delete_paths_are_restricted() {
        assert!(is_safe_remote_rel("config/BepInEx.cfg"));
        assert!(is_safe_remote_rel("plugins/ValheimPlus/ValheimPlus.dll"));
        assert!(!is_safe_remote_rel("config/../valheim_server.x86_64"));
        assert!(!is_safe_remote_rel("plugins/../../etc/passwd"));
        assert!(!is_safe_remote_rel("BepInEx/config/BepInEx.cfg"));
        assert!(!is_safe_remote_rel("startup_args.log"));
    }

    #[test]
    fn refresh_staging_copies_prunes_and_preserves_ignored() {
        let root = tempdir().unwrap();
        let source = root.path().join("source").join("BepInEx");
        let staging = root.path().join("staging");

        fs::create_dir_all(source.join("config")).unwrap();
        fs::create_dir_all(source.join("plugins")).unwrap();
        fs::write(source.join("config").join("a.cfg"), "a").unwrap();
        fs::write(source.join("plugins").join("b.dll"), "b").unwrap();
        fs::write(source.join("config").join("keep.state.json"), "ignored").unwrap();

        fs::create_dir_all(staging.join("config")).unwrap();
        fs::write(staging.join("config").join("obsolete.cfg"), "old").unwrap();
        fs::write(staging.join("config").join("server.state.json"), "keep").unwrap();

        let patterns = effective_patterns(&[]);
        let stats = refresh_staging(&source, &staging, &patterns).unwrap();

        assert_eq!(stats.copied, 2);
        assert_eq!(stats.pruned, 1);
        assert!(staging.join("config").join("a.cfg").exists());
        assert!(staging.join("plugins").join("b.dll").exists());
        assert!(!staging.join("config").join("obsolete.cfg").exists());
        assert!(staging.join("config").join("server.state.json").exists());
        assert!(!staging.join("config").join("keep.state.json").exists());
    }

    #[test]
    fn scan_and_state_roundtrip() {
        let root = tempdir().unwrap();
        let staging = root.path().join("staging");
        fs::create_dir_all(staging.join("config")).unwrap();
        fs::create_dir_all(staging.join("plugins")).unwrap();
        fs::write(staging.join("config").join("a.cfg"), "hello").unwrap();
        fs::write(staging.join("plugins").join("b.dll"), "world").unwrap();
        fs::write(staging.join("config").join("ignored.state.json"), "ignored").unwrap();
        fs::create_dir_all(staging.join("config").join("PUPSafeGraustenCombiner")).unwrap();
        fs::write(
            staging
                .join("config")
                .join("PUPSafeGraustenCombiner")
                .join("file.sub"),
            "ignored",
        )
        .unwrap();

        let patterns = effective_patterns(&[]);
        let hashes = scan_local_files(&staging, &patterns).unwrap();

        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains_key("config/a.cfg"));
        assert!(hashes.contains_key("plugins/b.dll"));
        assert!(!hashes
            .keys()
            .any(|path| path.contains("PUPSafeGraustenCombiner")));

        let path = state_path(&staging);
        save_state(&path, &hashes).unwrap();
        let loaded = load_state(&path).unwrap();
        assert_eq!(hashes, loaded);
    }
}
