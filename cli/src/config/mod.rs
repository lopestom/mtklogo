use mtklogo::ColorMode;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{Error as IOError, ErrorKind, Result};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub version: String,
    pub profiles: Vec<Profile>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Profile {
    pub name: String,
    pub color_model: String,
    pub alias: Option<Vec<String>>,
    pub formats: Vec<Format>,
}

impl Profile {
    pub fn with_color_model(self, color_model: String) -> Profile {
        Profile {
            name: self.name,
            color_model,
            alias: self.alias,
            formats: self.formats,
        }
    }
    pub fn guess_format(&self, size: u32, flip: bool) -> Result<Format> {
        let mtk_color_model = ColorMode::by_name(&self.color_model)?;
        let bpp = mtk_color_model.bytes_per_pixel();
        let pixels = size / bpp;
        let o = self.formats.iter().find(|f| f.w * f.h == pixels).cloned();
        match o {
            Some(f) => {
                if flip {
                    Ok(f.flip())
                } else {
                    Ok(f)
                }
            }
            None => Err(IOError::new(
                ErrorKind::InvalidData,
                format!(
                    "size '{}' does not correspond to any dimension in profile '{}'",
                    size, self.name
                ),
            )),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Format {
    pub w: u32,
    pub h: u32,
    pub t: Option<String>,
}

impl Format {
    pub fn flip(&self) -> Format {
        let flipped_title = self.t.as_ref().map(|s| format!("flip({})", s));
        Format {
            w: self.h,
            h: self.w,
            t: flipped_title,
        }
    }
}

impl Config {
    const GLOBAL_CONFIG: &'static str = "/etc/mtklogo.yaml";
    const RELATIVE_CONFIG: &'static str = "mtklogo.yaml";

    /// Resolves configuration path, in this order:
    /// - `"mtklogo.yaml"` in $HOME/.config
    /// - `"mtklogo.yaml"` in /etc
    /// - `"mtklogo.yaml"` in program's installation directory
    fn config_path() -> Result<(PathBuf, File)> {
        // in home directory?
        let home_config = {
            let mut home =
                env::home_dir().ok_or(IOError::new(ErrorKind::NotFound, "No home directory."))?;
            home.push(".config");
            home.push(Self::RELATIVE_CONFIG);
            File::open(home.as_path()).map(|f| (home, f))
        };
        // in /etc?
        let etc_config = {
            File::open(Config::GLOBAL_CONFIG).map(|f| (PathBuf::from(Config::GLOBAL_CONFIG), f))
        };
        // along with the executable?
        let shipped_config = {
            let self_dir = env::current_exe()?;
            let parent = self_dir.parent().ok_or(IOError::new(
                ErrorKind::NotFound,
                "Current executable is not inside a folder",
            ))?; /* seriously ? */
            let mut self_config = PathBuf::from(parent);
            self_config.push(Self::RELATIVE_CONFIG);
            File::open(self_config.as_path()).map(|f| (self_config, f))
        };
        home_config.or(etc_config).or(shipped_config).map_err(|_| {
            IOError::new(
                ErrorKind::NotFound,
                "`mtklogo.yaml` configuration not found, please provide one.",
            )
        })
    }

    fn wrap_read(path: &Path, file: File) -> Result<Config> {
        let config = serde_saphyr::from_reader(file);
        config.map_err(|e| {
            IOError::new(
                ErrorKind::InvalidData,
                format!("could not read config {} -> '{}'", path.display(), e),
            )
        })
    }

    pub fn from_file(path: &Path) -> Result<Config> {
        let file = File::open(path)?;
        Self::wrap_read(path, file)
    }

    pub fn load() -> Result<Config> {
        Self::load_with_path().map(|(config, _)| config)
    }

    /// Loads the configuration and also tells which file was actually used.
    ///
    /// `$HOME/.config/mtklogo.yaml` silently wins over the file shipped next
    /// to the executable, which is a classic trap: you edit the local yaml and
    /// nothing changes. Callers can show the path (and the shadowed files).
    pub fn load_with_path() -> Result<(Config, PathBuf)> {
        let (path, file) = Config::config_path()?;
        let config = Self::wrap_read(path.as_path(), file)?;
        Ok((config, path))
    }

    /// Other candidate configuration files that exist but are NOT used.
    pub fn shadowed_by(used: &Path) -> Vec<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(mut home) = env::home_dir() {
            home.push(".config");
            home.push(Self::RELATIVE_CONFIG);
            candidates.push(home);
        }
        candidates.push(PathBuf::from(Self::GLOBAL_CONFIG));
        if let Ok(exe) = env::current_exe()
            && let Some(parent) = exe.parent()
        {
            candidates.push(parent.join(Self::RELATIVE_CONFIG));
        }
        let used = std::fs::canonicalize(used).unwrap_or_else(|_| used.to_path_buf());
        candidates
            .into_iter()
            .filter(|c| c.is_file())
            .filter(|c| std::fs::canonicalize(c).map(|c| c != used).unwrap_or(true))
            .collect()
    }
}
