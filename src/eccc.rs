use chrono::{Datelike, DateTime, Utc, NaiveDate};
use errors::*;
use std::io::Read;
use std::str::{FromStr,from_utf8};
use reqwest;
use nom::digit;
#[cfg(test)]
use nom::{IResult};
use select::document::Document;
use select::predicate::{Attr, And, Class, Element, Name, Predicate};
use select::node::Node;
use types::{Source, Paper};

static ECCC: &'static str = "https://eccc.weizmann.ac.il/year/";
static BASE_URL: &'static str = "https://eccc.weizmann.ac.il";

///eccc has a custom format chrono cannot parse, so we build a parser with nom
named!(number<u32>, map_res!(map_res!(digit,from_utf8), FromStr::from_str) );
named!(eccc_rough_day <&[u8], u32>, do_parse!(v: number >> alt!(tag!("st") | alt!(tag!("nd") | tag!("rd") | tag!("th"))) >> (v)) );
named!(eccc_rough_month <&[u8], u32>, alt!(
    tag!("January")   => {|_| 1 } |
    tag!("February")  => {|_| 2 } |
    tag!("March")     => {|_| 3 } |
    tag!("April")     => {|_| 4 } |
    tag!("May")       => {|_| 5 } |
    tag!("June")      => {|_| 6 } |
    tag!("July")      => {|_| 7 } |
    tag!("August")    => {|_| 8 } |
    tag!("September") => {|_| 9 } |
    tag!("October")   => {|_| 10 } |
    tag!("November")  => {|_| 11 } |
    tag!("December")  => {|_| 12 }));
named!(eccc_rough_year <&[u8], u32>, do_parse!(v: number >> (v)));
named!(eccc_rough_date   <&[u8],NaiveDate>, do_parse!(
    day: eccc_rough_day >>
        tag!(" ") >>
        month: eccc_rough_month >>
        tag!(" ") >>
        year: eccc_rough_year >>
    (NaiveDate::from_ymd(year as i32, month, day))));


fn parse_rough_date(t: &str) -> Option<NaiveDate> {
    let st = t.trim();
    eccc_rough_date(st.as_bytes()).to_result().ok()
}

#[test]
fn date_parse_test() {
    //yes I am testing every month
    assert_eq!(parse_rough_date (" 16th November 2017"), Some(NaiveDate::from_ymd(2017,11,16)));
    assert_eq!(parse_rough_date (" 3rd November 2017"), Some(NaiveDate::from_ymd(2017,11,3)));
    assert_eq!(parse_rough_date (" 2nd November 2017"), Some(NaiveDate::from_ymd(2017,11,2)));
    assert_eq!(parse_rough_date (" 1st October 2017"), Some(NaiveDate::from_ymd(2017,10,1)));
    assert_eq!(parse_rough_date (" 26th September 2017"), Some(NaiveDate::from_ymd(2017,9,26)));
    assert_eq!(parse_rough_date (" 30th August 2017"), Some(NaiveDate::from_ymd(2017,8,30)));
    assert_eq!(parse_rough_date (" 28th July 2017"), Some(NaiveDate::from_ymd(2017,7,28)));
    assert_eq!(parse_rough_date (" 27th June 2017"), Some(NaiveDate::from_ymd(2017,6,27)));
    assert_eq!(parse_rough_date (" 28th May 2017"), Some(NaiveDate::from_ymd(2017,5,28)));
    assert_eq!(parse_rough_date (" 21st April 2017"), Some(NaiveDate::from_ymd(2017,4,21)));
    assert_eq!(parse_rough_date (" 26th March 2017"), Some(NaiveDate::from_ymd(2017,3,26)));
    assert_eq!(parse_rough_date (" 23rd February 2017"), Some(NaiveDate::from_ymd(2017,2,23)));
    assert_eq!(parse_rough_date (" 19th January 2017"), Some(NaiveDate::from_ymd(2017,1,19)));
}

/// The url we should look at. This is kind of rough as we just use the current year, hoping that nobody publishes around new year.
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

fn extract_id_rough_date(id_date_raw: String) -> (String, NaiveDate) {
    let mut id_vec = id_date_raw.split_whitespace().map(String::from).collect::<Vec<String>>();
    let date_vec = id_vec.split_off(2);
    let id_string = id_vec.iter().fold(String::from(""), |acc, x| acc + " " + &x);
    let date_string = date_vec.iter().fold(String::from(""), |acc, x| acc + " " + &x);
    let date_naive = parse_rough_date(&date_string).unwrap();
   
    (id_string, date_naive)
}

/// A rough paper gathers the information we can get from the summary page. Notice that the link and the date are not correct.
/// The link is a link to the details page
/// the date is just a date without any time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoughPaper {
    pub title: String,
    pub details_link: String,
    pub source: Source,
    pub rough_published: NaiveDate,
    pub authors: Vec<String>,
}

/// takes one div and returns the parsed rough paper
fn parse_single_div(div: Node) -> RoughPaper {
    let id_and_date_raw = div.find(Name("u")).nth(0).unwrap().text();
    let link_raw = div.find(Name("a")).nth(0).unwrap().attr("href").unwrap();
    let title_raw = div.find(Name("h4")).nth(0).unwrap().text();

    let authors_raw = div.children().nth(1).unwrap().text();
    
    let (id, date) = extract_id_rough_date(id_and_date_raw);
    let link = BASE_URL.to_owned() + link_raw.trim();
    let title = title_raw.trim();

    let authors = extract_authors(authors_raw);

    //we need to get the full abstract and full time from the details page
    RoughPaper { title: title.to_string(), details_link: link.to_string(), source: Source::ECCC, rough_published: date, authors: authors } 
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
    let divs = parsedoc.find(And(Name("div"),Attr("id", "box"))).take(1);
    
    //println!("{:?}", divs.nth(0).unwrap());
    
    let roughpapers = divs.map(parse_single_div).collect::<Vec<RoughPaper>>();    
    println!("{:?}", roughpapers);
    
    return Ok(vec![]);
    


}
