use crate::types::APP_INFO;
use anyhow::{Context, Result};
use app_dirs::*;
use chrono::TimeZone;
use std::fs::File;
use std::io::{Read, Write};

/// Struct representing a configuration.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// The Utc time when we last checked for new papers.
    last_checked: chrono::DateTime<chrono::Utc>,
    /// the google drive directory id.
    directory_id: String,
    /// ECCC time
    pub last_checked_eccc: Option<chrono::DateTime<chrono::Utc>>,
    /// Arxiv time
    pub last_checked_arxiv: Option<chrono::DateTime<chrono::Utc>>,
}

pub trait ConfigImpl {
    fn write(&self) -> Result<()>;
}

impl ConfigImpl for Config {
    fn write(&self) -> Result<()> {
        let mut path = app_root(AppDataType::UserConfig, &APP_INFO).expect("Error in app dir");
        path.push("config.toml");
        let mut file = File::create(path)?;
        let st = toml::to_string(&self);
        if st.is_err() {
            return Err(anyhow!("Could not parse toml"));
        }
        file.write_all(st.unwrap().as_bytes())
            .context("Cannot write")
    }
}

/// Reads the config file and returns the struct.
pub fn read_config_file() -> Result<Config> {
    let mut path = app_root(AppDataType::UserConfig, &APP_INFO).expect("Error in app dir");
    path.push("config.toml");
    let mut file = File::open(path)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    toml::from_str::<Config>(&s).context("Couldn't parse")
}

pub fn time_or_default(
    time: Option<chrono::DateTime<chrono::Utc>>,
) -> chrono::DateTime<chrono::Utc> {
    // we start with 2020 january first but eccc has stuff that we might miss, so it automatically removes a day.
    // Hence we have to be a day further.
    time.unwrap_or_else(|| chrono::Utc.ymd(2020, 1, 2).and_hms(0, 0, 1))
}

/// Reads the directory id from the config file and returns it.
pub fn read_directory_id() -> Result<String> {
    let c = read_config_file()?;
    Ok(c.directory_id)
}

/// Writes the current id to the config file. If the file does not exists, we abort and do not write.
pub fn write_directory_id(id: String) -> Result<()> {
    let mut c: Config = read_config_file()?;
    c.directory_id = id;
    c.write()
}

///  Writes the current id to the config file. If the file does not exists, we create it with default time and the given directory id.
pub fn write_directory_id_and_now(id: String) -> Result<()> {
    let c: Config = Config {
        last_checked: chrono::Utc::now(),
        directory_id: id,
        last_checked_arxiv: Some(chrono::Utc::now()),
        last_checked_eccc: Some(chrono::Utc::now()),
    };
    c.write()
}
