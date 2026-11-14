# oxifs 📁✨

A lightweight, high-performance file manager with an interactive disk storage visualizer, styled in **Catppuccin Mocha**. Built in Rust with [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-blue)

## Features

- **Blazing Fast**: Native desktop GUI with minimal memory footprint and instant startup.
- **Catppuccin Mocha Theme**: Cohesive dark aesthetic with distinct syntax/file-type accents.
- **Interactive Storage Visualizer**: Live disk usage breakdown and interactive pie chart of storage.
- **Keyboard Navigation**: Vim-friendly bindings (`j`, `k`, `h`, `Enter`) alongside mouse navigation.
- **Complete File Operations**: Copy (`y`), Cut (`x`), Paste (`p`), Rename (`r`), New Folder (`m`), and Safe Trash (`d`).
- **Live Search**: Instant filtering across current directories (`/`).
- **Wayland & X11 Native**: Built-in Wayland and X11 support via `winit` and `glow`.

---

## Keybindings

| Key | Action |
|:---:|:---|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `h` / `Backspace` | Navigate up to parent directory |
| `Enter` / `Click` | Open directory or launch file |
| `.` | Toggle hidden files |
| `/` | Focus search filter |
| `y` | Copy selected item |
| `x` | Cut selected item |
| `p` | Paste clipboard items into current folder |
| `r` | Rename selected item |
| `m` | Create new directory |
| `d` | Move selected item to Trash |
| `Y` | Yank / copy absolute path |
| `q` / `Esc` | Quit / clear search |

---

## Installation & Download

### Option 1: Precompiled Binary (Linux x86_64)

Download the release archive from GitHub Releases:

```bash
# 1. Extract archive
tar -xzf oxifs-v0.2.0-linux-x86_64.tar.gz

# 2. Install binary into ~/.local/bin
mkdir -p ~/.local/bin
mv oxifs ~/.local/bin/
chmod +x ~/.local/bin/oxifs

# 3. (Optional) Install desktop launcher
mkdir -p ~/.local/share/applications
cp oxifs.desktop ~/.local/share/applications/
```

Make sure `~/.local/bin` is in your `$PATH`.

### Option 2: Build from Source

Requirements: [Rust](https://www.rust-lang.org/) toolchain.

```bash
git clone https://github.com/<your-username>/oxifs.git
cd oxifs
cargo build --release
cp target/release/oxifs ~/.local/bin/
```

---

## License

MIT
