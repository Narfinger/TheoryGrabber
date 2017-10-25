extern crate cursive;
extern crate cursive_table_view;
extern crate rss;
extern crate url;


use std::fmt;
use cursive::Cursive;
use cursive::traits::*;
use cursive::align::HAlign::Center;
use cursive::align::HAlign;
use cursive::views::{Dialog, TextView};
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

impl TableViewItem<BasicColumn> for Paper {
    fn to_column(&self, column: BasicColumn) -> String {
        match column {
            BasicColumn::Title => self.title.to_owned(),
            BasicColumn::Source => format!("{}", self.source),
        }
    }

    fn cmp(&self, other: &Self, column: BasicColumn) -> std::cmp::Ordering where Self: Sized {
        match column {
            BasicColumn::Title => self.title.cmp(&other.title),
            BasicColumn::Source => self.source.cmp(&other.source),
        }
    }
}


#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum BasicColumn {
    Title,
    Source,
}

impl BasicColumn {
    fn as_str(&self) -> &str {
        match *self {
            BasicColumn::Title => "Title",
            BasicColumn::Source => "Source",
        }
    }

}

fn buildui() {
    
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
    let mut table = TableView::<Paper, BasicColumn>::new()
        .column(BasicColumn::Title, "Title", |c| c.ordering(std::cmp::Ordering::Greater))
        .column(BasicColumn::Source, "Source",|c| c.ordering(std::cmp::Ordering::Greater));
    
    table.set_items(papers);

    
    siv.add_layer(
        table
    );

    siv.run();
}
