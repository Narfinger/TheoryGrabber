use crate::types;
use crate::types::{Paper, Source};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Asia::Jerusalem;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha0, digit1};
use nom::combinator::{map, map_res, recognize};
use nom::number::complete::be_u32;
use nom::sequence::{self, tuple};
use nom::IResult;
#[cfg(test)]
use rayon::prelude::*;
use select::document::Document;
use select::node::Node;
use select::predicate::{And, Attr, Name};
use std::io::Read;

use nom::number::complete::u32;
static ECCC: &str = "https://eccc.weizmann.ac.il/year/";
static BASE_URL: &str = "https://eccc.weizmann.ac.il";

//eccc has a custom format chrono cannot parse, so we build a parser with nom
fn parse_u32(input: &str) -> IResult<&str, u32> {
    map_res(recognize(digit1), str::parse)(input)
}

fn parse_i32(input: &str) -> IResult<&str, i32> {
    map_res(recognize(digit1), str::parse)(input)
}

fn eccc_rough_day(input: &str) -> IResult<&str, u32> {
    let al = alt((tag("st"), tag("nd"), tag("rd")));
    let (rest, day) = tuple((al, parse_u32))(input)?;
    Ok(day)
}

fn eccc_rough_month(input: &str) -> IResult<&str, u32> {
    let p1 = map(tag("January"), |_| 1);
    let p2 = map(tag("February"), |_| 2);
    let p3 = map(tag("March"), |_| 3);
    let p4 = map(tag("April"), |_| 4);
    let p5 = map(tag("May"), |_| 5);
    let p6 = map(tag("June"), |_| 6);
    let p7 = map(tag("July"), |_| 7);
    let p8 = map(tag("August"), |_| 8);
    let p9 = map(tag("September"), |_| 9);
    let p10 = map(tag("October"), |_| 10);
    let p11 = map(tag("November"), |_| 11);
    let p12 = map(tag("December"), |_| 12);

    alt((p1, p2, p3, p4, p5, p6, p7, p8, p9, p10, p11, p12))(input)
}

fn eccc_rough_date(input: &str) -> IResult<&str, NaiveDate> {
    let (rest, (day, month, year)) = tuple((eccc_rough_day, eccc_rough_month, parse_i32))(input)?;
    Ok((rest, NaiveDate::from_ymd(year, month, day)))
}

/// Parses a date from the overview page.
fn parse_rough_date(t: &str) -> Result<NaiveDate> {
    let st = t.trim().to_owned() + " ";
    let res = eccc_rough_date(&st);
    if let Ok((_, s)) = res {
        Ok(s)
    } else {
        Err(anyhow!(
            "Error parsing rough date from string \"{}\"\n with error {:?}",
            &st,
            res
        ))
    }
}

fn eccc_date_time(input: &str) -> IResult<&str, NaiveDateTime> {
    let (input, (day, month, year, hour, _, minute)) = tuple((
        eccc_rough_day,
        eccc_rough_month,
        parse_i32,
        parse_u32,
        tag(":"),
        parse_u32,
    ))(input)?;
    Ok((
        input,
        NaiveDate::from_ymd(year, month, day).and_hms(hour, minute, 0),
    ))
}

/// This uses Israel Standard time at the moment. This might change when eccc moves.
fn parse_date(t: &str) -> Result<DateTime<Utc>> {
    let st = t.trim().to_owned() + " ";
    let res = eccc_date_time(&st);
    if let Ok((_, naive)) = res {
        if let LocalResult::Single(israel) = Jerusalem.from_local_datetime(&naive) {
            Ok(israel.with_timezone(&Utc))
        } else {
            Err(anyhow!("timezone conversion failed"))
        }
    } else {
        Err(anyhow!(
            "error in parsing date from \"{}\" with error {:?}",
            st,
            res
        ))
    }
}

#[test]
fn date_rough_parse_test() {
    //yes I am testing every month
    assert_eq!(
        parse_rough_date(" 16th November 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 11, 16))
    );
    assert_eq!(
        parse_rough_date(" 3rd November 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 11, 3))
    );
    assert_eq!(
        parse_rough_date(" 2nd November 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 11, 2))
    );
    assert_eq!(
        parse_rough_date(" 1st October 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 10, 1))
    );
    assert_eq!(
        parse_rough_date(" 26th September 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 9, 26))
    );
    assert_eq!(
        parse_rough_date(" 30th August 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 8, 30))
    );
    assert_eq!(
        parse_rough_date(" 28th July 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 7, 28))
    );
    assert_eq!(
        parse_rough_date(" 27th June 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 6, 27))
    );
    assert_eq!(
        parse_rough_date(" 28th May 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 5, 28))
    );
    assert_eq!(
        parse_rough_date(" 21st April 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 4, 21))
    );
    assert_eq!(
        parse_rough_date(" 26th March 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 3, 26))
    );
    assert_eq!(
        parse_rough_date(" 23rd February 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 2, 23))
    );
    assert_eq!(
        parse_rough_date(" 19th January 2017").ok(),
        Some(NaiveDate::from_ymd(2017, 1, 19))
    );
}

#[test]
fn date_parse_test() {
    //parsing without timezone
    //assert_eq!(eccc_date_time(&b"16th November 2017 04:24"[..]), IResult::Done(&b""[..], NaiveDate::from_ymd(2017,11,16).and_hms(4,24,0)));

    //with timezone
    assert_eq!(
        parse_date(" 16th November 2017 04:24").ok(),
        Some(Utc.ymd(2017, 11, 16).and_hms(2, 24, 0))
    );
}

/// The url we should look at. This is kind of rough as we just use the current year, hoping that nobody publishes around new year.
fn get_url() -> String {
    let now = Utc::now().year();

    format!("{}{:?}/", ECCC.to_string(), now)
}

/// This extracts the authors.
/// We split at whitespace, skip the first 5 which are id and link
/// we construct enumerates for all authors, to merge adjacent authors together, i.e., ["Foo" "Bar"] -> ["Foo Bar"].
fn extract_authors(authors_raw: &str) -> Vec<String> {
    type EnumeratedVec = (Vec<(usize, String)>, Vec<(usize, String)>);

    let enumerated_strings = authors_raw
        .split_whitespace()
        .skip(5)
        .map(String::from)
        .enumerate();
    let (even, odd): EnumeratedVec = enumerated_strings.partition(|&(x, _)| x % 2 == 0);

    even.into_iter()
        .zip(odd)
        .map(|((_, x), (_, y))| x + " " + &y)
        .collect::<Vec<String>>()
}

/// This extracts the id and rough date from the `id_date_raw` string.
fn extract_id_rough_date(id_date_raw: &str) -> Result<(String, NaiveDate)> {
    let mut id_vec = id_date_raw
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<String>>();
    let date_vec = id_vec.split_off(2);
    let id_string = id_vec.iter().fold(String::from(""), |acc, x| acc + " " + x);
    let date_string = date_vec
        .iter()
        .fold(String::from(""), |acc, x| acc + " " + x);
    let date_naive = parse_rough_date(&date_string)?;

    Ok((id_string, date_naive))
}

/// A rough paper gathers the information we can get from the summary page.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RoughPaper {
    /// The title of the paper.
    pub title: String,
    /// The link to the details page, not the link to the paper pdf.
    pub details_link: String,
    /// The date when the ppaer was published. This does not include a time.
    pub rough_published: NaiveDate,
    /// The authors of the paper.
    pub authors: Vec<String>,
}

/// Converts a `RoughPaper` to a `Paper` with the missing fields given.
fn to_paper(
    p: &RoughPaper,
    link: reqwest::Url,
    description: String,
    published: DateTime<Utc>,
) -> Paper {
    Paper {
        title: types::sanitize_title(&p.title),
        source: Source::ECCC,
        authors: p.authors.to_owned(),
        link,
        description,
        published,
    }
}

/// Takes one div and returns the parsed `RoughPaper`.
fn parse_single_div(div: Node) -> Result<RoughPaper> {
    //testing if the unwraps are ok
    //normally I would use different way to do this but I don't quite know how to make it work here
    if div.find(Name("u")).next().is_none() {
        return Err(anyhow!("Could not find u name"));
    }
    if div.find(Name("a")).next().is_none() {
        return Err(anyhow!("Could not find a name"));
    }
    if div.find(Name("a")).next().unwrap().attr("href").is_none() {
        return Err(anyhow!("a name does not have href"));
    }
    if div.find(Name("h4")).next().is_none() {
        return Err(anyhow!("could not find h4"));
    }

    if div.children().nth(1).is_none() {
        return Err(anyhow!("authors not found"));
    }

    let id_and_date_raw = div.find(Name("u")).next().unwrap().text();
    let link_raw = div.find(Name("a")).next().unwrap().attr("href").unwrap();
    let title_raw = div.find(Name("h4")).next().unwrap().text();

    let authors_raw = div.children().nth(1).unwrap().text();

    let (_, date) = extract_id_rough_date(&id_and_date_raw)?;
    let link = BASE_URL.to_owned() + link_raw.trim();
    let title = title_raw.trim();

    let authors = extract_authors(&authors_raw);

    //we need to get the full abstract and full time from the details page

    Ok(RoughPaper {
        title: title.to_string(),
        details_link: link,
        rough_published: date,
        authors,
    })
}

/// Parses the year summary page.
/// This function needs a major overhau las how we handle vectors is really rough
fn parse_eccc_summary() -> Result<Vec<RoughPaper>> {
    let string = get_url();
    let res = {
        let res = reqwest::blocking::get(&string).context("Can't get eccc")?;
        if !res.status().is_success() {
            return Err(anyhow!("Some error in getting the reqwuest"));
        }
        let parsedoc = Document::from_read(res)?;
        let divs = parsedoc.find(And(Name("div"), Attr("id", "box")));

        //Vec<Result<Vec<Result>>> is the corresponding type structure, a bit
        divs.map(parse_single_div)
            .collect::<Result<Vec<RoughPaper>>>() //type: Vec<Result<RoughPaper>> (actually iterator over the vector)
    };
    res
}

/// Function that queries and parses the details page for a given `RoughPaper`.
fn parse_eccc_details(p: &RoughPaper) -> Result<Paper> {
    let mut res = reqwest::blocking::get(&p.details_link).context("Can't get eccc")?;
    if !res.status().is_success() {
        return Err(anyhow!("Some error in getting the reqwuest"));
    }

    let mut res_string = String::new();
    res.read_to_string(&mut res_string)?;

    let parsedoc = Document::from(res_string.as_str());
    let div = parsedoc
        .find(And(Name("div"), Attr("id", "box")))
        .next()
        .unwrap();

    let id_and_date_text = div.children().nth(1).unwrap().text();
    //format is TR17-177 | 16th November 2017 04:24 \n\n {Title}, we skip the id part and add back the whitespaces
    let date_string = id_and_date_text
        .split('|')
        .nth(1)
        .unwrap()
        .split_whitespace()
        .take(4)
        .fold(String::from(""), |acc, x| acc + " " + x);
    let date = parse_date(&date_string).unwrap();

    let url_string = p.details_link.to_owned() + "/download";
    let link = reqwest::Url::parse(&url_string).unwrap(); //we do not need to parse anything

    //this check is for parser security as we just take an arbitrary child which is not very parsing save
    if div.children().nth(8).unwrap().text() != "Abstract:" {
        return Err(anyhow!("Cannot find abstract"));
    }

    //another "safety" check that it indeed has a <p> tag
    let description = div
        .children()
        .nth(11)
        .unwrap()
        .first_child()
        .unwrap()
        .text();

    Ok(to_paper(p, link, description, date))
}

/// This parses the whole ECCC by first looking at the current year, getting these details and filtering and then parsing the details pages to construct full papers.
pub fn parse_eccc(utc: DateTime<Utc>) -> Result<Vec<Paper>> {
    let rough_papers = parse_eccc_summary()?;
    // the -1 should not be necessary but we might loose a day in the utc to naivedate conversion.
    // wrong results will be removed in the second filter
    let days = if utc.day() == 1 {
        //if we ask on the first, we obviously cannot go negative.
        1
    } else {
        utc.day() - 1
    };
    let naive_filter_date = NaiveDate::from_ymd(utc.year(), utc.month(), days);

    info!("Found rough papers: {:?}", &rough_papers);

    rough_papers
        .iter()
        .filter(|p| p.rough_published >= naive_filter_date)
        .map(parse_eccc_details)
        .filter(|p| {
            if let Ok(p) = p {
                p.published >= utc
            } else {
                true
            }
        })
        .collect::<Result<Vec<Paper>>>()
}

#[test]
fn parse_eccc_test() {
    //let now = Utc::now();
    //assert!(false);
    //assert!(parse_eccc(now).is_ok());
}
