use chrono;
use chrono::TimeZone;
use errors::*;
use std::fs::File;
use std::io::Write;
use serde_yaml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    last_checked: chrono::DateTime<chrono::Utc>,
    directory_id: String,
}

fn read_config_file() -> Result<Config> {
    let file = File::open("config.yaml")?;
    serde_yaml::from_reader::<File, Config>(file).chain_err(|| "Couldn't parse")
}

fn write_config_file(c: Config) -> Result<()> {
    let mut file = File::create("config.yaml")?;
    let st = serde_yaml::to_string(&c)?;
    file.write_all(st.as_bytes()).chain_err(|| "Cannot write")
}

fn read_config_time() -> Result<chrono::DateTime<chrono::Utc>> {
    let c = read_config_file()?;
    Ok(c.last_checked)
}

pub fn read_config_time_or_default() -> chrono::DateTime<chrono::Utc> {
    read_config_time().unwrap_or_else(|_| chrono::Utc.ymd(1985, 1, 1).and_hms(0, 0, 1))
}

fn write_config_time(time: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let mut c = read_config_file()?;
    c.last_checked = time;
    write_config_file(c).chain_err(|| "Couldn't write time")
}

pub fn write_now() -> Result<()> {
    write_config_time(chrono::Utc::now()).chain_err(|| "Cannot write current time")
}

pub fn read_directory_id() -> Result<String> {
    let c = read_config_file()?;
    Ok(c.directory_id)
}

pub fn write_directory_id(id: String) -> Result<()> {
    let mut c: Config =
        if let Ok(c) = read_config_file() {
            c
        } else {
            Config { last_checked: chrono::Utc.ymd(1985, 1, 1).and_hms(0, 0, 1), directory_id: String::from("") }
        };
    c.directory_id = id;
    write_config_file(c)
}
