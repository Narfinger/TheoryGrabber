#![recursion_limit = "128"]
extern crate app_dirs;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate anyhow;
extern crate chrono;
extern crate chrono_tz;
extern crate cursive;
extern crate cursive_table_view;
extern crate indicatif;
extern crate rayon;
extern crate reqwest;
extern crate select;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate quick_xml;
extern crate tempdir;
extern crate toml;
extern crate url;
#[macro_use]
extern crate nom;
extern crate oauth2;
extern crate tokio_core;

pub mod arxiv;
pub mod config;
pub mod drive;
pub mod drive_oauth;
pub mod eccc;
pub mod gui;
pub mod paper_dialog;
pub mod types;

use crate::types::{DownloadedPaper, Paper};
use anyhow::{Context, Result};
use clap::{App, Arg};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::copy;
use std::path::Path;
use std::thread;
use tempdir::TempDir;

/// Downloads the papers to the `TempDir`.
fn download_papers<'a>(papers: &'a [Paper], dir: &TempDir) -> Result<Vec<DownloadedPaper<'a>>> {
    let mut files: Vec<DownloadedPaper> = Vec::new();
    let progressbar = ProgressBar::new(papers.len() as u64);
    progressbar.set_message("Downloading Papers");
    progressbar.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed_precise}] {msg} {spinner:.green} {bar:100.green/blue} {pos:>7}/{len:7}",
            )
            .progress_chars("#>-"),
    );
    progressbar.enable_steady_tick(100);

    let client = reqwest::blocking::Client::builder()
        .user_agent("TheoryGrabber")
        .build()?;
    for i in progressbar.wrap_iter(papers.iter()) {
        let mut response = client.get(i.link.clone()).send()?;
        let contenttype = response.headers().get(reqwest::header::CONTENT_TYPE);
        if contenttype
            != Some(
                &reqwest::header::HeaderValue::from_str("application/pdf")
                    .expect("Error in parsing header"),
            )
        {
            return Err(anyhow!(
                "We got the wrong content type for {}, type: {:?}",
                &i.link,
                contenttype
            ));
        }
        let filename = Path::new("").with_file_name(&i.title).with_extension("pdf");
        let savefile = dir.path().join(&filename);
        let mut file = File::create(savefile.clone()).context(anyhow!(
            "Error in file with filename: {:?}, savefile: {:?}",
            filename.clone(),
            savefile.clone()
        ))?;
        copy(&mut response, &mut file)?;

        files.push(DownloadedPaper {
            paper: i,
            path: savefile,
        });
    }
    progressbar.finish();

    Ok(files)
}

/// Fetches the papers and filters for new papers.
fn get_and_filter_papers() -> Result<Vec<Paper>> {
    let utc = config::read_config_time_or_default();

    let progress_fetch_bar = ProgressBar::new_spinner();
    progress_fetch_bar.set_message("Getting Arxiv/ECCC");
    progress_fetch_bar.set_style(
        ProgressStyle::default_bar().template("[{elapsed_precise}] {msg} {spinner:.green}"),
    );
    progress_fetch_bar.enable_steady_tick(100);

    let handle = thread::spawn(|| arxiv::parse_arxiv().context("Error in parsing arxiv papers"));

    let mut eccc_papers = eccc::parse_eccc(utc).context("Error in parsing eccc")?;
    let mut papers = handle.join().unwrap()?; //I do not like that we have to use unwrap here but I do not know how to use ? with error_chain with these types

    papers.append(&mut eccc_papers);

    let mut papers_filtered = types::filter_papers(papers, utc);
    progress_fetch_bar.set_message("Sorting and Deduping");
    papers_filtered.sort_unstable();
    types::dedup_papers(&mut papers_filtered);

    progress_fetch_bar.finish_with_message("Done fetching");

    Ok(papers_filtered)
}

fn setup() -> Result<(oauth2::basic::BasicTokenResponse, String)> {
    let tk = drive_oauth::setup_oauth()?;
    let directory_id = if let Ok(id) = config::read_directory_id() {
        id
    } else {
        let id = drive::create_directory(&tk)?;
        config::write_directory_id_and_now(id.clone())?;
        id
    };

    Ok((tk, directory_id))
}

fn run() -> Result<()> {
    println!("Ideas for improvement:");
    println!("sub implement dialog for deleting to support delete key and stuff");
    println!("better error handling for resumeable downloads\n");

    let (tk, directory_id) = setup()?;

    let filtered_papers = get_and_filter_papers()?;

    if filtered_papers.is_empty() {
        println!("Nothing new found. Saving new date.");
        return config::write_now();
    }

    if let Some(papers_to_download) = gui::get_selected_papers(filtered_papers) {
        if papers_to_download.is_empty() {
            println!("No papers to download");
            return config::write_now();
        }

        if let Ok(dir) = TempDir::new("TheoryGrabber") {
            let files = download_papers(&papers_to_download, &dir).context("Files error")?;

            let progressbar = ProgressBar::new(files.len() as u64);
            progressbar.set_message("Uploading Papers");
            progressbar.set_style(ProgressStyle::default_bar()
                                  .template("[{elapsed_precise}] {msg} {spinner:.green} {bar:100.green/blue} {pos:>7}/{len:7}")
                                  .progress_chars("#>-"));
            progressbar.enable_steady_tick(100);

            for i in progressbar.wrap_iter(files.iter()) {
                let f = File::open(i.path.clone()).context("File couldn't be opened")?;
                drive::upload_file(&tk, f, i.paper, &directory_id)
                    .context("Uploading function has error")?;
            }
            //progressbar.finish();
            //config::write_now()?;
        }
    } else {
        println!("Nothing to download and we are not saving.");
    }
    Ok(())
}

fn main() {
    let matches = App::new("TheoryGrabber")
        .version(crate_version!())
        .author("Narfinger")
        .about("Grabs ArXiv and ECCC papers, lists them, downloads them and uploads them to a folder in Google Drive.")
        .arg(Arg::with_name("clean")
             .short("c")
             .long("clean")
             .help("Sets the current date as a date in the config. This is should only be used before the first download \
                    to not make you go through papers you already know."))
        .get_matches();

    if matches.is_present("clean") {
        let res = setup();
        if res.is_err() {
            println!("Could not write file, something is wrong.");
        }
    }

    run().expect("Error");
}
