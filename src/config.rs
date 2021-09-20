use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};

/// Struct representing a configuration.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// The Utc time when we last checked for new papers.
    last_checked: chrono::DateTime<chrono::Utc>,
    /// the google drive directory id.
    pub directory_id: Option<String>,
    /// ECCC time
    pub last_checked_eccc: Option<chrono::DateTime<chrono::Utc>>,
    /// Arxiv time
    pub last_checked_arxiv: Option<chrono::DateTime<chrono::Utc>>,
    pub local_store: Option<String>,
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
    pub fn write(&self) -> Result<()> {
        let mut path = crate::types::get_config_dir()?;
        path.push("config.toml");
        let mut file = File::create(path)?;
        let st = toml::to_string(&self);
        if st.is_err() {
            return Err(anyhow!("Could not parse toml"));
        }
        file.write_all(st.unwrap().as_bytes())
            .context("Cannot write")
    }

    pub fn read() -> Result<Config> {
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

pub fn read_directory_id() -> Option<String> {
    Config::read().ok().and_then(|c| c.directory_id)
}

pub fn write_directory_id_and_now(id: String) -> Result<()> {
    let mut c = Config::read_or_default();
    c.directory_id = Some(id);
    c.last_checked = chrono::Utc::now();
    c.write()
}
