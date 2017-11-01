extern crate chrono;
extern crate cursive;
extern crate cursive_table_view;
extern crate indicatif;
extern crate reqwest;
extern crate rss;
extern crate url;
extern crate quick_xml;


use std::fmt;
use std::io::Read;
use chrono::{DateTime, Utc};
use cursive::Cursive;
use cursive::align::HAlign::Center;
use cursive::align::HAlign;
use cursive::views::{Dialog, LinearLayout, TextView};
use cursive_table_view::{TableView, TableViewItem};
use cursive::traits::*;
use indicatif::ProgressBar;
use rss::Channel;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use url::Url;


static ARXIV: &'static str = "http://export.arxiv.org/api/query?search_query=cat:cs.CC&sortBy=lastUpdatedDate&sortOrder=descending&max_results=100";
static ECCC: &'static str = "http://eccc.hpi-web.de/feeds/reports/";

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum Source {
    Arxiv,
    ECCC,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out = match *self {
            Source::Arxiv => "ArXiV",
            Source::ECCC => "ECCC",
        };

        write!(f, "{}", out)
    }
}


#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum BasicColumn {
    Title,
    Source,
    Published,
}

impl BasicColumn {
    fn as_str(&self) -> &str {
        match *self {
            BasicColumn::Title => "Title",
            BasicColumn::Source => "Source",
            BasicColumn::Published => "Published",
        }
    }

}

#[derive(Clone, Debug)]
struct Paper {
    title: String,
    description: String,
    link: url::Url,
    source: Source, 
    published: i64, //in unix epoch
}

impl TableViewItem<BasicColumn> for Paper {
    fn to_column(&self, column: BasicColumn) -> String {
        match column {
            BasicColumn::Title => self.title.to_owned(),
            BasicColumn::Source => format!("{}", self.source),
            BasicColumn::Published => format!("{}", self.published),
        }
    }

    fn cmp(&self, other: &Self, column: BasicColumn) -> std::cmp::Ordering where Self: Sized {
        match column {
            BasicColumn::Title     => self.title.cmp(&other.title),
            BasicColumn::Source    => self.source.cmp(&other.source),
            BasicColumn::Published => self.published.cmp(&other.published),
        }
    }
}

fn parse_arxiv() -> Vec<Paper> {
    enum Tag {Entry, Published, Title, Summary, Link, Nothing};
    struct TmpPaper { published: Option<String>, title: Option<String>, summary: Option<String>, link: Option<String>};


    //this is super inefficient!
    let mut reqreader = reqwest::get(ARXIV).unwrap();
    let mut resp = String::new();
    reqreader.read_to_string(&mut resp);


    
    let mut reader = Reader::from_str(resp.as_str());
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut tag = Tag::Nothing;
    let mut entries: Vec<TmpPaper> = Vec::new();
    let mut cur_paper: TmpPaper = TmpPaper { published: None, title: None, summary: None, link: None };
    loop {    
        match reader.read_event(&mut buf) {
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"entry" => {tag = Tag::Nothing; entries.push(cur_paper); cur_paper = TmpPaper { published: None, title: None, summary: None, link: None};},
                    _ =>        {tag = Tag::Entry;  },
                }
            },
            Ok(Event::Start(ref e)) =>
                match e.name() {
                    b"entry"     => {tag = Tag::Entry; cur_paper = TmpPaper{published: None, title: None, summary: None, link: None};},
                    b"published" => {tag = Tag::Published;},
                    b"title"     => {tag = Tag::Title;},
                    b"summary"   => {tag = Tag::Summary;},
                    b"link"      => {tag = Tag::Link;},
                    _ =>            {},
                },
            Ok(Event::Text(e)) => {
                match tag {
                    Tag::Published  => { cur_paper.published = Some(e.unescape_and_decode(&reader).unwrap())}, 
                    Tag::Title      => { cur_paper.title     = Some(e.unescape_and_decode(&reader).unwrap())}, 
                    Tag::Summary    => { cur_paper.summary   = Some(e.unescape_and_decode(&reader).unwrap())},
                    Tag::Link       => { cur_paper.link      = Some(e.unescape_and_decode(&reader).unwrap())},
                    Tag::Nothing | Tag::Entry => {},  
                }


                e.unescape_and_decode(&reader).unwrap();},
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
             _ => (), // There are several other `Event`s we do not consider here
        }
    }

    //converting tmp to real and testing
    entries.into_iter().map(|p|
                            Paper { title: p.title.unwrap(),
                                    description: p.summary.unwrap(),
                                    published: 0 /* p.published */,
                                    link: Url::parse(p.link.unwrap().as_str()).unwrap(),
                                    source: Source::Arxiv,
                            }
    ).collect::<Vec<Paper>>()
}

fn buildui() {
    
}

fn main() {
    let res = parse_arxiv();
    println!("{:?}", res);



    //let pb = ProgressBar::new_spinner();
    //pb.enable_steady_tick(100);
    
    let channel = Channel::from_url(ARXIV).unwrap();

    let papers = channel
        .items()
        .into_iter()
        .map(|i| {
            let title = i.title().unwrap();
            let description = i.description().unwrap();
            let link = i.link().unwrap();
            println!("{}", i.pub_date().unwrap());
            let published = DateTime::parse_from_rfc2822(i.pub_date().unwrap_or("")).unwrap();

            Paper {
                title: title.to_string(),
                description: description.to_string(),
                link: Url::parse(link).unwrap(),
                source: Source::Arxiv,
                published: published.timestamp(),
            }
        }).collect::<Vec<Paper>>();


    let mut siv = Cursive::new();
    let mut table = TableView::<Paper, BasicColumn>::new()
        .column(BasicColumn::Title, "Title", |c| c.ordering(std::cmp::Ordering::Greater))
        .column(BasicColumn::Source, "Source",|c| c.ordering(std::cmp::Ordering::Greater))
        .column(BasicColumn::Published, "Published", |c| c.ordering(std::cmp::Ordering::Greater));
    
    table.set_items(papers);

    siv.add_layer(
        Dialog::around(table.with_id("table").min_size((50, 20)))
            .title("Table View")
            .button("Quit", |s| s.quit())
    );

    //pb.finish_and_clear();
    
    siv.run();
}
