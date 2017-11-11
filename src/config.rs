use chrono;
use chrono::TimeZone;
use errors::*;
use std::fs::File;
use std::io::Write;
use serde_yaml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    last_checked: chrono::DateTime<chrono::Utc>,
}

fn read_config_time() -> Result<chrono::DateTime<chrono::Utc>> {
    let file = File::open("config.yaml")?;
    let c = serde_yaml::from_reader::<File, Config>(file)?;
    Ok(c.last_checked)
}

pub fn read_config_time_or_default() -> chrono::DateTime<chrono::Utc> {
    read_config_time().unwrap_or_else(|_| chrono::Utc.ymd(1985, 1, 1).and_hms(0, 0, 1))
}

fn write_config_time(time: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let mut file = File::create("config.yaml")?;
    let c = Config { last_checked: time };
    //serde_yaml::to_writer(&c, &mut file)?
    let st = serde_yaml::to_string(&c)?;
    file.write_all(st.as_bytes()).chain_err(|| "Cannot write")
}

pub fn write_now() -> Result<()> {
    write_config_time(chrono::Utc::now()).chain_err(|| "Cannot write current time")
}
