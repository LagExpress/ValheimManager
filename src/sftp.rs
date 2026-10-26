use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use ssh2::{FileStat, HashType, Session, Sftp};

use crate::sync::is_ignored;

pub const REMOTE_ROOT: &str = "/BepInEx";
pub const WRAPPER_PATH: &str = "/valheim_server.x86_64";
pub const WRAPPER_BIN_PATH: &str = "/valheim_server.x86_64.bin";
pub const DATA_LINK_PATH: &str = "/valheim_server.x86_64_Data";
pub const DATA_TARGET: &str = "valheim_server_Data";
pub const WRAPPER_SCRIPT: &str = include_str!("assets/valheim_server_wrapper.sh");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapperStatus {
    Intact,
    ElfNeedsRename,
    Missing,
    #[default]
    Unknown,
}

impl WrapperStatus {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            WrapperStatus::Intact => "wrapper intact",
            WrapperStatus::ElfNeedsRename => "game binary needs rename",
            WrapperStatus::Missing => "wrapper missing",
            WrapperStatus::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WrapperReport {
    pub status: WrapperStatus,
    pub installed: bool,
    pub renamed_elf: bool,
    pub symlink_created: bool,
    pub symlink_present: bool,
    pub warnings: Vec<String>,
}

pub struct RemoteClient {
    sftp: Sftp,
    _session: Session,
}

impl RemoteClient {
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(Self, Option<String>), String> {
        let host = host.trim();
        let username = username.trim();
        let addrs = (host, port)
            .to_socket_addrs()
            .map_err(|e| format!("Could not resolve {}:{}: {}", host, port, e))?;

        let mut stream = None;
        let mut last_error = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, Duration::from_secs(15)) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => last_error = Some(e),
            }
        }

        let stream = stream.ok_or_else(|| {
            format!(
                "Could not connect to {}:{}: {}",
                host,
                port,
                last_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "no addresses resolved".to_string())
            )
        })?;

        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));

        let mut session =
            Session::new().map_err(|e| format!("Could not create SSH session: {}", e))?;
        session.set_tcp_stream(stream);
        session.set_timeout(15000);
        session
            .handshake()
            .map_err(|e| format!("SSH handshake failed: {}", e))?;
        session
            .userauth_password(username, password)
            .map_err(|e| format!("Password authentication failed: {}", e))?;

        if !session.authenticated() {
            return Err("Password authentication failed".to_string());
        }

        let fingerprint = session.host_key_hash(HashType::Sha256).map(hex_bytes);
        let sftp = session
            .sftp()
            .map_err(|e| format!("Could not open SFTP subsystem: {}", e))?;

        Ok((
            Self {
                sftp,
                _session: session,
            },
            fingerprint,
        ))
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), String> {
        let stat = FileStat {
            size: None,
            uid: None,
            gid: None,
            perm: Some(mode),
            atime: None,
            mtime: None,
        };
        self.sftp
            .setstat(Path::new(path), stat)
            .map_err(|e| format!("Could not set permissions on {}: {}", path, e))
    }

    pub fn list_dir_names(&self, dir: &str) -> Result<Vec<String>, String> {
        let entries = self
            .sftp
            .readdir(Path::new(dir))
            .map_err(|e| format!("Could not list {}: {}", dir, e))?;

        Ok(entries
            .into_iter()
            .filter_map(|(path, _)| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| *name != "." && *name != "..")
                    .map(|name| name.to_string())
            })
            .collect())
    }

    fn root_names(&self) -> Result<Vec<String>, String> {
        self.list_dir_names("/")
    }

    fn read_header(&self, path: &str) -> Result<Vec<u8>, String> {
        let mut file = self
            .sftp
            .open(Path::new(path))
            .map_err(|e| format!("Could not open {}: {}", path, e))?;
        let mut buffer = [0u8; 16];
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Could not read {}: {}", path, e))?;
        Ok(buffer[..read].to_vec())
    }

    fn write_wrapper(&self) -> Result<(), String> {
        let mut file = self
            .sftp
            .create(Path::new(WRAPPER_PATH))
            .map_err(|e| format!("Could not create {}: {}", WRAPPER_PATH, e))?;
        file.write_all(WRAPPER_SCRIPT.as_bytes())
            .map_err(|e| format!("Could not write {}: {}", WRAPPER_PATH, e))?;
        drop(file);
        self.chmod(WRAPPER_PATH, 0o755)
    }

    pub fn ensure_dir(&self, dir: &str) -> Result<(), String> {
        let mut current = String::new();
        for part in dir.trim_matches('/').split('/') {
            if part.is_empty() {
                continue;
            }
            current.push('/');
            current.push_str(part);

            if self.sftp.stat(Path::new(&current)).is_err() {
                self.sftp
                    .mkdir(Path::new(&current), 0o755)
                    .map_err(|e| format!("Could not create remote folder {}: {}", current, e))?;
            }
        }
        Ok(())
    }

    pub fn list_files(
        &self,
        dir: &str,
        base_rel: &str,
        patterns: &[String],
    ) -> Result<Vec<String>, String> {
        if self.sftp.stat(Path::new(dir)).is_err() {
            return Ok(Vec::new());
        }

        let entries = self
            .sftp
            .readdir(Path::new(dir))
            .map_err(|e| format!("Could not list remote folder {}: {}", dir, e))?;

        let mut files = Vec::new();
        for (path, stat) in entries {
            let name = match path.file_name().and_then(|name| name.to_str()) {
                Some(name) if name != "." && name != ".." => name.to_string(),
                _ => continue,
            };
            let rel = if base_rel.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", base_rel, name)
            };

            if is_ignored(&rel, patterns) {
                continue;
            }

            if stat.is_dir() {
                let child = format!("{}/{}", dir.trim_end_matches('/'), name);
                files.extend(self.list_files(&child, &rel, patterns)?);
            } else {
                files.push(rel);
            }
        }

        Ok(files)
    }

    pub fn upload(&self, local: &Path, remote: &str) -> Result<(), String> {
        let mut source = File::open(local)
            .map_err(|e| format!("Could not open local file {}: {}", local.display(), e))?;
        let mut target = self
            .sftp
            .create(Path::new(remote))
            .map_err(|e| format!("Could not create remote file {}: {}", remote, e))?;

        std::io::copy(&mut source, &mut target)
            .map_err(|e| format!("Could not upload {}: {}", remote, e))?;
        drop(target);
        self.chmod(remote, 0o644)
    }

    pub fn remove_file(&self, remote: &str) -> Result<(), String> {
        self.sftp
            .unlink(Path::new(remote))
            .map_err(|e| format!("Could not delete {}: {}", remote, e))
    }

    pub fn clean_empty_dirs(&self, start_dir: &str, patterns: &[String]) -> Result<(), String> {
        let stop = REMOTE_ROOT;
        let mut current = start_dir.trim_end_matches('/').to_string();

        while current != stop && current.starts_with(&format!("{}/", stop)) {
            let rel = current[stop.len() + 1..].to_string();
            if is_ignored(&rel, patterns) {
                break;
            }

            let entries = match self.sftp.readdir(Path::new(&current)) {
                Ok(entries) => entries,
                Err(_) => break,
            };

            let empty = entries.iter().all(|(path, _)| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "." || name == "..")
            });
            if !empty {
                break;
            }

            if self.sftp.rmdir(Path::new(&current)).is_err() {
                break;
            }

            current = match current.rsplit_once('/') {
                Some((parent, _)) if !parent.is_empty() => parent.to_string(),
                _ => break,
            };
        }

        Ok(())
    }

    pub fn inspect_wrapper(&self) -> Result<WrapperReport, String> {
        let names = self.root_names()?;
        let mut report = WrapperReport {
            symlink_present: names
                .iter()
                .any(|name| name == "valheim_server.x86_64_Data"),
            ..Default::default()
        };

        if names.iter().any(|name| name == "valheim_server.x86_64") {
            match self.read_header(WRAPPER_PATH) {
                Ok(header) if header.starts_with(b"\x7fELF") => {
                    report.status = WrapperStatus::ElfNeedsRename;
                }
                Ok(header) if header.starts_with(b"#!") => {
                    report.status = WrapperStatus::Intact;
                }
                Ok(_) => {
                    report.status = WrapperStatus::Unknown;
                }
                Err(e) => {
                    report.status = WrapperStatus::Unknown;
                    report.warnings.push(e);
                }
            }
        } else {
            report.status = WrapperStatus::Missing;
        }

        Ok(report)
    }

    pub fn check_and_fix_wrapper(&self) -> Result<WrapperReport, String> {
        let names = self.root_names()?;
        let mut report = WrapperReport::default();
        let mut needs_install = false;

        if names.iter().any(|name| name == "valheim_server.x86_64") {
            match self.read_header(WRAPPER_PATH) {
                Ok(header) if header.starts_with(b"\x7fELF") => {
                    report.status = WrapperStatus::ElfNeedsRename;
                    report.renamed_elf = true;
                    let _ = self.sftp.unlink(Path::new(WRAPPER_BIN_PATH));
                    self.sftp
                        .rename(Path::new(WRAPPER_PATH), Path::new(WRAPPER_BIN_PATH), None)
                        .map_err(|e| format!("Could not rename the game binary: {}", e))?;
                    self.chmod(WRAPPER_BIN_PATH, 0o755)?;
                    needs_install = true;
                }
                Ok(header) if header.starts_with(b"#!") => {
                    report.status = WrapperStatus::Intact;
                }
                Ok(_) => {
                    report.status = WrapperStatus::Unknown;
                    needs_install = true;
                }
                Err(e) => {
                    report.status = WrapperStatus::Unknown;
                    report.warnings.push(e);
                    needs_install = true;
                }
            }
        } else {
            report.status = WrapperStatus::Missing;
            needs_install = true;
        }

        if needs_install {
            self.write_wrapper()?;
            report.installed = true;
            report.status = WrapperStatus::Intact;
        }

        if names
            .iter()
            .any(|name| name == "valheim_server.x86_64_Data")
        {
            report.symlink_present = true;
        } else {
            match self
                .sftp
                .symlink(Path::new(DATA_TARGET), Path::new(DATA_LINK_PATH))
            {
                Ok(()) => {
                    report.symlink_created = true;
                    report.symlink_present = true;
                }
                Err(e) => report
                    .warnings
                    .push(format!("Could not create the data symlink: {}", e)),
            }
        }

        Ok(report)
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}
