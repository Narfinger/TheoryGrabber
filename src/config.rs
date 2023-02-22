use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

/// Struct representing a configuration.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Config {
    /// The Utc time when we last checked for new papers.
    last_checked: chrono::DateTime<chrono::Utc>,
    /// the google drive directory id.
    pub(crate) directory_id: Option<String>,
    /// ECCC time
    pub(crate) last_checked_eccc: Option<chrono::DateTime<chrono::Utc>>,
    /// Arxiv time
    pub(crate) last_checked_arxiv: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) local_store: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            last_checked: chrono::Utc::now(),
            directory_id: None,
            last_checked_arxiv: Some(chrono::Utc::now()),
            last_checked_eccc: Some(chrono::Utc::now()),
            local_store: None,
        }
    }
}

impl Config {
    pub(crate) fn write(&self) -> Result<()> {
        let mut path = crate::types::get_config_dir()?;
        path.push("config.toml");
        if !path.exists() {
            println!("Creating directories for {:?}", &path);
            std::fs::create_dir_all(path.parent().unwrap())
                .context("trying to create directory for config file")?;
        }
        let mut file = File::create(path)?;
        let st = toml::to_string(&self);
        if st.is_err() {
            return Err(anyhow!("Could not parse toml"));
        }
        file.write_all(st.unwrap().as_bytes())
            .context("Cannot write")
    }

    pub(crate) fn read() -> Result<Config> {
        let mut path = crate::types::get_config_dir()?;
        path.push("config.toml");
        let mut file = File::open(path)?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        toml::from_str::<Config>(&s).context("Couldn't parse")
    }

    pub fn read_or_default() -> Config {
        Config::read().unwrap_or_else(|_| Config::default())
    }
}

pub(crate) fn read_directory_id() -> Option<String> {
    Config::read().ok().and_then(|c| c.directory_id)
}
