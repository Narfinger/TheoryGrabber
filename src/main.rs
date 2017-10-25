extern crate cursive;
extern crate cursive_table_view;
extern crate rss;
extern crate url;


use std::fmt;
use cursive::Cursive;
use cursive::traits::*;
use cursive::align::HAlign::Center;
use cursive::align::HAlign;
use cursive::views::{LinearLayout, ListView, Dialog, TextView};
use cursive_table_view::{TableView, TableViewItem};
use rss::Channel;
use url::Url;


static ARXIV: &'static str = "http://arxiv.org/rss/cs.CC";
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

#[derive(Clone, Debug)]
struct Paper {
    title: String,
    description: String,
    link: url::Url,
    source: Source, 
}

impl fmt::Display for Paper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.title, self.source)
    }
}

fn main() {
    let channel = Channel::from_url(ARXIV).unwrap();

    let papers = channel
        .items()
        .into_iter()
        .map(|i| {
            let title = i.title().unwrap();
            let description = i.description().unwrap();
            let link = i.link().unwrap();

            Paper {
                title: title.to_string(),
                description: description.to_string(),
                link: Url::parse(link).unwrap(),
                source: Source::Arxiv,
            }
        }).collect::<Vec<Paper>>();


    let mut siv = Cursive::new();
    let mut l = ListView::new();
    for i in papers {
        let mut tv = TextView::new(format!("{}", i));
        l.add_child(i.title.as_str(), tv);
    }
    
    siv.add_layer(Dialog::around(l));

    siv.run();
}
