use chrono::{Datelike, Utc};
use errors::*;
use std::io::Read;
use reqwest;
use select::document::Document;
use select::predicate::{Attr, And, Class, Element, Name, Predicate};
use types::Paper;


static ECCC: &'static str = "https://eccc.weizmann.ac.il/year/";

fn get_url() -> String {
    ECCC.to_string() + &Utc::now().year().to_string() + "/"
}

pub fn parse_eccc() -> Result<Vec<Paper>> {
    let mut res = reqwest::get(get_url().as_str()).chain_err(|| "Can't get eccc")?;
    if !res.status().is_success() {
        return Err("Some error in getting the reqwuest".into());
    } 

    
    let mut res_string = String::new();
    res.read_to_string(&mut res_string)?;
    //println!("{}", get_url());
    
    let parsedoc = Document::from(res_string.as_str());
    //println!("res_string: {}", res_string);
    let mut divs = parsedoc.find(And(Name("div"),Attr("id", "box")));
    
    println!("{:?}", divs.nth(0).unwrap());
    
    
    
    return Ok(vec![]);
    


}
