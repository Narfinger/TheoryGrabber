#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![recursion_limit="128"]
extern crate chrono;
extern crate cursive;
extern crate cursive_table_view;
#[macro_use]
extern crate error_chain;
extern crate indicatif;
extern crate reqwest;
extern crate rss;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate url;
extern crate quick_xml;

mod errors {
    error_chain!{
        foreign_links {
            Io(::std::io::Error);
            Serde(::serde_yaml::Error);
        }
    }
}

pub mod arxiv;
pub mod config;
pub mod types;

use cursive::Cursive;
use cursive::views::{Dialog, LinearLayout, TextView, DummyView};
use cursive_table_view::{TableView, TableViewItem};
use cursive::traits::*;
use indicatif::ProgressBar;
use types::{Paper, print_authors};
use arxiv::parse_arxiv;

static ECCC: &'static str = "http://eccc.hpi-web.de/feeds/reports/";

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

impl TableViewItem<BasicColumn> for Paper {
    fn to_column(&self, column: BasicColumn) -> String {
        match column {
            BasicColumn::Title => {
                let mut res = self.title.replace("\n", "").to_owned();
                res.truncate(120);
                res
            }
            BasicColumn::Source => format!("{}", self.source),
            BasicColumn::Published => format!("{}", self.published),
        }
    }

    fn cmp(&self, other: &Self, column: BasicColumn) -> std::cmp::Ordering
        where Self: Sized
    {
        match column {
            BasicColumn::Title => self.title.cmp(&other.title),
            BasicColumn::Source => self.source.cmp(&other.source),
            BasicColumn::Published => self.published.cmp(&other.published),
        }
    }
}

fn buildui() {}

fn main() {
    let papers = parse_arxiv();
    //println!("{:?}", res);



    //let pb = ProgressBar::new_spinner();
    //pb.enable_steady_tick(100);

    // let channel = Channel::from_url(ARXIV).unwrap();

    // let papers = channel
    //     .items()
    //     .into_iter()
    //     .map(|i| {
    //         let title = i.title().unwrap();
    //         let description = i.description().unwrap();
    //         let link = i.link().unwrap();
    //         println!("{}", i.pub_date().unwrap());
    //         let published = DateTime::parse_from_rfc2822(i.pub_date().unwrap_or("")).unwrap();

    //         Paper {
    //             title: title.to_string(),
    //             description: description.to_string(),
    //             link: Url::parse(link).unwrap(),
    //             source: Source::Arxiv,
    //             published: published.timestamp(),
    //         }
    //     })
    //     .collect::<Vec<Paper>>();


    let mut siv = Cursive::new();
    let mut table = TableView::<Paper, BasicColumn>::new()
        .column(BasicColumn::Title,
                "Title",
                |c| c.ordering(std::cmp::Ordering::Greater).width_percent(75))
        .column(BasicColumn::Source,
                "Source",
                |c| c.ordering(std::cmp::Ordering::Greater))
        .column(BasicColumn::Published,
                "Published",
                |c| c.ordering(std::cmp::Ordering::Greater));

    table.set_items(papers);
    table.set_on_submit(|siv: &mut Cursive, row: usize, index: usize| {

        let value: Paper = siv.call_on_id("table",
                                          move |table: &mut TableView<Paper, BasicColumn>| {
                                              table.borrow_item(index).unwrap().clone()
                                          })
            .unwrap();

        let d = LinearLayout::vertical()
            .child(TextView::new(value.title.clone()))
            .child(DummyView)
            .child(TextView::new(print_authors(&value)))
            .child(DummyView)
            .child(TextView::new(value.description.clone()));

        siv.add_layer(Dialog::around(d)
                          .title(format!("Removing row # {}", row))
                          .button("Delete", move |s| {
            s.call_on_id("table",
                         |table: &mut TableView<Paper, BasicColumn>| { table.remove_item(index); });
            s.pop_layer()
        })
                          .button("Close", move |s| s.pop_layer()));
    });

    siv.add_layer(Dialog::around(table.with_id("table").min_size((500, 80)))
                      .title("Table View")
                      .button("Quit", |s| s.quit()));

    //pb.finish_and_clear();

    siv.run();
}
