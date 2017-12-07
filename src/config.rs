use chrono;
use chrono::TimeZone;
use errors::*;
use std::fs::File;
use std::io::Write;
use serde_yaml;

/// Struct representing a configuration.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// The Utc time when we last checked for new papers.
    last_checked: chrono::DateTime<chrono::Utc>,
    /// the google drive directory id.
    directory_id: String,
}

/// Reads the config file and returns the struct.
fn read_config_file() -> Result<Config> {
    let file = File::open("config.yaml")?;
    serde_yaml::from_reader::<File, Config>(file).chain_err(|| "Couldn't parse")
}

/// writes the config file to the current directory config.yaml.
fn write_config_file(c: &Config) -> Result<()> {
    let mut file = File::create("config.yaml")?;
    let st = serde_yaml::to_string(&c)?;
    file.write_all(st.as_bytes()).chain_err(|| "Cannot write")
}

/// Read config time from config file.
fn read_config_time() -> Result<chrono::DateTime<chrono::Utc>> {
    let c = read_config_file()?;
    Ok(c.last_checked)
}

/// Reads the time in the config file or returns a default time (1.1.1985, 00:00:01).
pub fn read_config_time_or_default() -> chrono::DateTime<chrono::Utc> {
    read_config_time().unwrap_or_else(|_| chrono::Utc.ymd(1985, 1, 1).and_hms(0, 0, 1))
}

/// Writes the config file. If no config file is found, we do not write the current time!
fn write_config_time(time: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let mut c = read_config_file()?;
    c.last_checked = time;
    write_config_file(&c).chain_err(|| "Couldn't write time")
}

/// Writes the current time to the config file. If no config file is found, we do not write the current time!
pub fn write_now() -> Result<()> {
    write_config_time(chrono::Utc::now()).chain_err(|| "Cannot write current time")
}

/// Reads the directory id from the config file and returns it.
pub fn read_directory_id() -> Result<String> {
    let c = read_config_file()?;
    Ok(c.directory_id)
}

/// Writes the current id to the config file. If the file does not exists, we create it with default time and the given directory id.
pub fn write_directory_id(id: String) -> Result<()> {
    let mut c: Config =
        if let Ok(c) = read_config_file() {
            c
        } else {
            Config { last_checked: read_config_time_or_default(), directory_id: String::from("") }
        };
    c.directory_id = id;
    write_config_file(&c)
}
