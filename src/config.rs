use chrono;
use errors::*;
use std::fs::File;
use std::io::Write;
use serde_yaml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    last_checked: chrono::DateTime<chrono::Utc>,
}

pub fn read_config_time() -> Result<chrono::DateTime<chrono::Utc>> {
    let file = File::open("config.yaml")?;
    let c = serde_yaml::from_reader::<File, Config>(file)?;
    Ok(c.last_checked)
}

pub fn write_config_time(time: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let mut file = File::create("config.yaml")?;
    let c = Config { last_checked: time };
    //serde_yaml::to_writer(&c, &mut file)?
    let st = serde_yaml::to_string(&c)?;
    file.write_all(st.as_bytes()).chain_err(|| "Cannot write")
}
