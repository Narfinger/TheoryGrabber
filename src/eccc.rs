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
    let mut divs = parsedoc.find(And(Name("div"),Attr("id", "box"))).take(1);
    
    //println!("{:?}", divs.nth(0).unwrap());

    for div in divs {
        let id_and_date_raw = div.find(Name("u")).nth(0).unwrap().text();
        let link_raw = div.find(Name("a")).nth(0).unwrap().text();
        let title_raw = div.find(Name("h4")).nth(0).unwrap().text();
        let abs_raw = div.find(And(Name("div"), Attr("style", "text-align:justify;"))).nth(0).unwrap().text();
        let authors_raw = div.text();

        let id_and_date = id_and_date_raw.trim();
        let link = link_raw.trim();
        let title = title_raw.trim();
        let abs = abs_raw.trim();
        let authors = authors_raw.trim();

        
        println!("Parsed: idd{:?}, link{:?}, title{:?}, abs{:?} authors{:?}", id_and_date, link, title, abs, authors);
    } 
    
    
    
    return Ok(vec![]);
    


}
