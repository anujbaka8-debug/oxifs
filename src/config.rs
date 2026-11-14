use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub theme: Theme,
    pub keys: Keys,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub selection: String,
    pub dir_color: String,
    pub symlink_color: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Keys {
    pub quit: char,
    pub up: char,
    pub down: char,
    pub enter: char,
    pub go_up: char,
    pub delete: char,
    pub open: char,
    pub toggle_hidden: char,
    pub search: char,
    pub copy: char,
    pub cut: char,
    pub paste: char,
    pub rename: char,
    pub mkdir: char,
    pub yank_path: char,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: Theme {
                bg: "#1e1e2e".into(),
                fg: "#cdd6f4".into(),
                accent: "#f5c2e7".into(),
                selection: "#313244".into(),
                dir_color: "#89b4fa".into(),
                symlink_color: "#a6e3a1".into(),
            },
            keys: Keys {
                quit: 'q',
                up: 'k',
                down: 'j',
                enter: '\n',
                go_up: 'h',
                delete: 'd',
                open: 'o',
                toggle_hidden: '.',
                search: '/',
                copy: 'y',
                cut: 'x',
                paste: 'p',
                rename: 'r',
                mkdir: 'm',
                yank_path: 'Y',
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            toml::from_str(&text).unwrap_or_default()
        } else {
            Config::default()
        }
    }
}

fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut h = dirs_home();
            h.push(".config");
            h
        });
    base.join("oxifs").join("config.toml")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
