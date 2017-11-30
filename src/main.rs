#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![recursion_limit="128"]
extern crate chrono;
extern crate chrono_tz;
extern crate cursive;
extern crate cursive_table_view;
#[macro_use]
extern crate error_chain;
extern crate indicatif;
extern crate reqwest;
extern crate rss;
extern crate select;
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate tempdir;
extern crate url;
extern crate quick_xml;
extern crate hyper;
extern crate hyper_rustls;
#[macro_use]
extern crate nom;
extern crate tokio_core;
extern crate yup_oauth2 as oauth2;

mod errors {
    error_chain!{
        foreign_links {
            Io(::std::io::Error);
            Serde(::serde_yaml::Error);
            Reqwest(::reqwest::Error);
            Nom(::nom::ErrorKind);
        }
    }
}

pub mod arxiv;
pub mod config;
pub mod drive;
pub mod eccc;
pub mod gui;
pub mod paper_dialog;
pub mod types;

use errors::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::thread;
use std::io::copy;
use std::fs::File;
use std::path::Path;
use types::{DownloadedPaper, Paper};
use tempdir::TempDir;

fn download_papers<'a>(papers: &'a [Paper], dir: &TempDir) -> Result<Vec<DownloadedPaper<'a>>> {
    let mut files: Vec<DownloadedPaper> = Vec::new();
    let progressbar = ProgressBar::new(papers.len() as u64);
    progressbar.set_message("Downloading Papers");
    progressbar.set_style(ProgressStyle::default_bar()
                          .template("[{elapsed_precise}] {msg} {spinner:.green} {bar:100.green/blue} {pos:>7}/{len:7}")
                          .progress_chars("#>-"));
    progressbar.enable_steady_tick(100);

    for i in progressbar.wrap_iter(papers.iter()) {
        let sanitized_title = i.title.replace("\n", "").replace("  ", " ");
        let mut response = reqwest::get(i.link.clone())?;
        let filename = Path::new("")
            .with_file_name(sanitized_title)
            .with_extension("pdf");
        let savefile = dir.path().join(filename);
        //        println!("{:?}",savefile);
        let mut file = File::create(savefile.clone()).unwrap();
        copy(&mut response, &mut file)?;

        files.push(DownloadedPaper {
            paper: i,
            path: savefile,
        });
    }
    progressbar.finish();

    Ok(files)
}

fn get_and_filter_papers() -> Result<Vec<Paper>> {
    let utc = config::read_config_time_or_default();

    let progress_fetch_bar = ProgressBar::new_spinner();
    progress_fetch_bar.set_message("Getting Arxiv/ECCC");
    progress_fetch_bar.set_style(ProgressStyle::default_bar()
                          .template("[{elapsed_precise}] {msg} {spinner:.green}"));
    progress_fetch_bar.enable_steady_tick(100);

    let handle = thread::spawn(|| {
        arxiv::parse_arxiv().chain_err(|| "Error in parsing arxiv papers")
    });
    
    let mut eccc_papers = eccc::parse_eccc(utc).chain_err(|| "Error in parsing eccc")?;
    let mut papers = handle.join().unwrap()?; //I do not like that we have to use unwrap here but I do not know how to use ? with error_chain with these types
    
    papers.append(&mut eccc_papers);

    let mut papers_filtered = types::filter_papers(papers, utc);
    progress_fetch_bar.set_message("Sorting and Deduping");
    papers_filtered.sort_unstable();
    types::dedup_papers(&mut papers_filtered);
    
    progress_fetch_bar.finish_with_message("Done fetching");
    
    Ok(papers_filtered)
}


fn run() -> Result<()> {
    println!("Ideas for improvement:");
    println!("sub implement dialog for deleting to support delete key and stuff");
    println!("better error handling for resumeable downloads\n");
    
    let tk = drive::setup_oauth2();
    let directory_id = if let Ok(id) = config::read_directory_id() {
        id
    } else {
        drive::create_directory(&tk)?;
        config::read_directory_id()?
    };

    let filtered_papers = get_and_filter_papers()?;

    if filtered_papers.is_empty() {
        println!("Nothing new found. Saving new date.");
        return config::write_now()
    }
    
    if let Some(papers_to_download) = gui::get_selected_papers(filtered_papers) {
        if papers_to_download.is_empty() {
            println!("No papers to download");
            return config::write_now()
        }

        
        if let Ok(dir) = TempDir::new("TheoryGrabber") {
            let files = download_papers(&papers_to_download, &dir).chain_err(
                || "Files error",
            )?;
            
            let progressbar = ProgressBar::new(files.len() as u64);
            progressbar.set_message("Uploading Papers");
            progressbar.set_style(ProgressStyle::default_bar()
                                  .template("[{elapsed_precise}] {msg} {spinner:.green} {bar:100.green/blue} {pos:>7}/{len:7}")
                                  .progress_chars("#>-"));
            progressbar.enable_steady_tick(100);

            for i in progressbar.wrap_iter(files.iter()) {
                let f = File::open(i.path.clone()).chain_err(
                    || "File couldn't be opened",
                )?;
                drive::upload_file(&tk, f, i.paper, directory_id.clone()).chain_err(
                    || "Uploading function has error",
                )?;
            }
            progressbar.finish();
            config::write_now()?;
        }
    } else {
        println!("Nothing to download and we are not saving.");
    }
    Ok(())
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
    }
}
