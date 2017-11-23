use chrono::{Datelike, DateTime, Utc};
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

/// this extracts the authors in the following way
/// we split at whitespace, skip the first 5 which are id and link
/// we construct enumerates for all authors, to merge adjacent authors together, i.e., ["Foo" "Bar"] -> ["Foo Bar"]
fn extract_authors(authors_raw: String) -> Vec<String> {
    let enumerated_strings = authors_raw.split_whitespace().skip(5).map(String::from).enumerate();
    let (even, odd): (Vec<(usize, String)>, Vec<(usize, String)>) = enumerated_strings.partition(|&(x,ref el)| x % 2 == 0);
    let authors = even.into_iter().zip(odd).map(|((_,x),(_,y))| x + " " + &y).collect::<Vec<String>>();

    authors
}

/// The full abstract is in the details so we better filter before we look into the details of a million papers. Hence this function needs a filter time
pub fn parse_eccc(utc: DateTime<Utc>) -> Result<Vec<Paper>> {
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
        let link_raw = div.find(Name("a")).nth(0).unwrap().attr("href").unwrap();
        let title_raw = div.find(Name("h4")).nth(0).unwrap().text();
//        let abs_raw = div.find(And(Name("div"), Attr("style", "text-align:justify;"))).nth(0).unwrap().text();
        let authors_raw = div.children().nth(1).unwrap().text();

        let id_and_date = id_and_date_raw.trim();
        let link = link_raw.trim();
        let title = title_raw.trim();
//        let abs = abs_raw.trim();
        let authors = extract_authors(authors_raw);


        //we need to get the full abstract from the details page
        //println!("authors: {}", authors);
        println!("Parsed: idd{:?}, link{:?}, title{:?}, authors{:?}", id_and_date, link, title, authors);
    } 
    
    
    
    return Ok(vec![]);
    


}
