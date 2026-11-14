use anyhow::Result;
use chrono::{DateTime, Local};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
}

#[allow(dead_code)]
impl FileEntry {
    pub fn size_mb(&self) -> f64 {
        self.size as f64 / 1_048_576.0
    }
    pub fn size_gb(&self) -> f64 {
        self.size as f64 / 1_073_741_824.0
    }
}

pub enum FsMsg {
    DirLoaded(PathBuf, Vec<FileEntry>),
    DirSizesComputed(PathBuf, Vec<(PathBuf, u64)>),
    OpDone(String),
    OpError(String),
}

pub fn load_dir(dir: PathBuf, show_hidden: bool, tx: Sender<FsMsg>) {
    let dir_clone = dir.clone();
    let tx_clone = tx.clone();
    std::thread::spawn(move || {
        let entries = walk(&dir, show_hidden);
        let subdirs: Vec<PathBuf> = entries
            .iter()
            .filter(|e| e.is_dir && !e.is_symlink)
            .map(|e| e.path.clone())
            .collect();

        let _ = tx.send(FsMsg::DirLoaded(dir.clone(), entries));

        // Background calculation of subfolder sizes (skip for root / or system mount roots to stay instant)
        if dir != Path::new("/") && !dir.starts_with("/proc") && !dir.starts_with("/sys") && !subdirs.is_empty() {
            let mut sizes = Vec::new();
            for sub in subdirs {
                let s: u64 = WalkBuilder::new(&sub)
                    .max_depth(Some(3))
                    .hidden(true)
                    .follow_links(false)
                    .build()
                    .filter_map(|r| r.ok())
                    .filter_map(|e| e.metadata().ok())
                    .filter(|m| m.is_file())
                    .map(|m| m.len())
                    .sum();
                sizes.push((sub, s));
            }
            let _ = tx_clone.send(FsMsg::DirSizesComputed(dir_clone, sizes));
        }
    });
}

fn walk(dir: &Path, show_hidden: bool) -> Vec<FileEntry> {
    let mut v: Vec<FileEntry> = WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(!show_hidden)
        .follow_links(false)
        .build()
        .filter_map(|r| r.ok())
        .filter(|e| e.path() != dir)
        .map(|e| {
            let path = e.path().to_path_buf();
            let meta = e.metadata().ok();
            let is_symlink = e.path_is_symlink();
            let is_dir = meta.as_ref().map_or(false, |m| m.is_dir());
            let size = meta.as_ref().map_or(0, |m| m.len());
            let modified = meta
                .and_then(|m| m.modified().ok())
                .map(DateTime::<Local>::from);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            FileEntry { name, path, is_dir, is_symlink, size, modified }
        })
        .collect();

    v.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    v
}

pub fn trash_path(path: PathBuf, tx: Sender<FsMsg>) {
    std::thread::spawn(move || {
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        match trash::delete(&path) {
            Ok(()) => { let _ = tx.send(FsMsg::OpDone(format!("Trashed: {name}"))); }
            Err(e) => { let _ = tx.send(FsMsg::OpError(e.to_string())); }
        }
    });
}

pub fn copy_path(src: PathBuf, dest: PathBuf, tx: Sender<FsMsg>) {
    std::thread::spawn(move || match copy_r(&src, &dest) {
        Ok(()) => { let _ = tx.send(FsMsg::OpDone("Copied.".into())); }
        Err(e) => { let _ = tx.send(FsMsg::OpError(e.to_string())); }
    });
}

pub fn move_path(src: PathBuf, dest: PathBuf, tx: Sender<FsMsg>) {
    std::thread::spawn(move || {
        let do_move = || -> Result<()> {
            if std::fs::rename(&src, &dest).is_err() {
                copy_r(&src, &dest)?;
                rm_all(&src)?;
            }
            Ok(())
        };
        match do_move() {
            Ok(()) => { let _ = tx.send(FsMsg::OpDone("Moved.".into())); }
            Err(e) => { let _ = tx.send(FsMsg::OpError(e.to_string())); }
        }
    });
}

fn copy_r(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dest)?;
        for e in std::fs::read_dir(src)? {
            let e = e?;
            copy_r(&e.path(), &dest.join(e.file_name()))?;
        }
    } else {
        std::fs::copy(src, dest)?;
    }
    Ok(())
}

fn rm_all(path: &Path) -> Result<()> {
    std::fs::remove_dir_all(path)
        .or_else(|_| std::fs::remove_file(path))?;
    Ok(())
}

pub fn human_size(bytes: u64) -> String {
    const GB: f64 = 1_073_741_824.0;
    const MB: f64 = 1_048_576.0;
    const KB: f64 = 1_024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_mb(bytes: u64) -> String {
    format!("{:.2} MB", bytes as f64 / 1_048_576.0)
}

pub fn format_gb(bytes: u64) -> String {
    format!("{:.3} GB", bytes as f64 / 1_073_741_824.0)
}
