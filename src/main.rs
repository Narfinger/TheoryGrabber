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
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate tempdir;
extern crate url;
extern crate quick_xml;
extern crate hyper;
extern crate hyper_rustls;
extern crate tokio_core;
extern crate yup_oauth2 as oauth2;

mod errors {
    error_chain!{
        foreign_links {
            Io(::std::io::Error);
            Serde(::serde_yaml::Error);
            Reqwest(::reqwest::Error);
        }
    }
}

pub mod arxiv;
pub mod config;
pub mod drive;
pub mod types;

use cursive::Cursive;
use cursive::views::{Dialog, LinearLayout, TextView, DummyView};
use cursive_table_view::{TableView, TableViewItem};
use cursive::traits::*;
use errors::*;
use indicatif::ProgressBar;
use std::io::{copy, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::{thread, time};
use types::{Paper, print_authors};
use tempdir::TempDir;
use arxiv::parse_arxiv;
use drive::{setup_oauth2, upload_file};

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

fn download_papers(papers: &[Paper], dir: &TempDir) -> Result<Vec<File>> {
    let mut files: Vec<File>  = Vec::new();
    let progressbar = ProgressBar::new(papers.len() as u64);
    for i in papers {
        progressbar.inc(1);
        let sanitized_title = i.title.replace("\n", "").replace("  ", " ");
        let mut response = reqwest::get(i.link.clone())?;
        let filename = Path::new("").with_file_name(sanitized_title).with_extension("pdf");
        let savefile = dir.path().join(filename);
//        println!("{:?}",savefile);
        let mut file = File::create(savefile).unwrap();
        copy(&mut response, &mut file)?;

        files.push(file);
    }
    progressbar.finish_and_clear();

    Ok(files)
}

fn build_gui(papers: Vec<Paper>) -> Vec<Paper> {
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
            .child(TextView::new(value.link.clone().to_string()))
            .child(DummyView)
            .child(TextView::new(value.description.clone()));

        siv.add_layer(Dialog::around(d)
                      .title(format!("Removing row # {}", row))
                      .button("Delete", move |s| {
                          s.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
                              table.remove_item(index);
                          });
                          s.pop_layer()
                      }).button("Close", move |s| s.pop_layer()));
    });

    siv.add_layer(Dialog::around(table.with_id("table").min_size((500, 80)))
                  .title("Table View")
                  .button("Download all", move |s| {
                      s.quit()})
                  .button("Quit", move |s| {
                      s.call_on_id("table", move |table: &mut TableView<Paper, BasicColumn>| {
                          table.clear();
                      });
                      s.quit();
                  })
    );

    //pb.finish_and_clear();

    siv.run();
    siv.call_on_id("table", |table: &mut TableView<Paper, BasicColumn>| {
        table.take_items()
    }).unwrap()
}

fn run() -> Result<()> {
    let papers = parse_arxiv().chain_err(|| "Error in parsing papers")?;
    let utc = config::read_config_time_or_default();
    let filtered_papers = types::filter_papers(papers, utc);
    //currently disabled
    //config::write_now();
    
    let filtered_papers = build_gui(filtered_papers);
    if let Ok(dir) = TempDir::new("TheoryGrabber") {
        let files = download_papers(&filtered_papers, &dir).chain_err(|| "Files error")?;

        let ten_millis = time::Duration::from_millis(100000);
        thread::sleep(ten_millis);
        println!("Trying to upload first file");

        let mut file = files.first().chain_err(|| "Not first found")?;
        let tk = setup_oauth2();

        file.seek(SeekFrom::Start(0)).chain_err(|| "Error in seeking to start")?;
        let fcopy = file.try_clone().chain_err(|| "Cloning is wrong?")?;
        upload_file(&tk, fcopy).chain_err(|| "Uploading function has error")?;
    }
    Ok(())
    
    //    println!("{:?}", filtered_papers);
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}
