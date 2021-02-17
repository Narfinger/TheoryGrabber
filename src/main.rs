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
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
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
extern crate console;
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
use std::sync::Arc;
use std::sync::RwLock;
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
        info!("Downloading {} onto {:?}", &i.title, &savefile);
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
fn get_and_filter_papers(config: &Arc<RwLock<config::Config>>) -> Result<Vec<Paper>> {
    let progress_fetch_bar = ProgressBar::new_spinner();
    progress_fetch_bar.set_message("Getting Arxiv/ECCC");
    progress_fetch_bar.set_style(
        ProgressStyle::default_bar().template("[{elapsed_precise}] {msg} {spinner:.green}"),
    );
    progress_fetch_bar.enable_steady_tick(100);

    let c = config.clone();
    let handle = thread::spawn(move || {
        let time = c
            .read()
            .unwrap()
            .last_checked_arxiv
            .unwrap_or(chrono::Utc::now());
        let arxiv_papers = arxiv::parse_arxiv().map(|p| types::filter_papers(p, time));
        if let Ok(mut arxiv_papers) = arxiv_papers {
            arxiv_papers.sort_unstable();
            if let Some(ref p) = arxiv_papers.last() {
                c.write().unwrap().last_checked_arxiv =
                    p.published.checked_add_signed(chrono::Duration::minutes(1));
            }
            Ok(arxiv_papers)
        } else {
            arxiv_papers
        }
    });
    let eccc_time = config
        .read()
        .unwrap()
        .last_checked_eccc
        .unwrap_or(chrono::Utc::now());
    let eccc_papers = eccc::parse_eccc(eccc_time).context("Error in parsing eccc")?;
    let mut eccc_papers = types::filter_papers(eccc_papers, eccc_time);
    eccc_papers.sort_unstable();
    if let Some(l) = eccc_papers.last().map(|p| p.published) {
        config.write().unwrap().last_checked_eccc =
            l.checked_add_signed(chrono::Duration::minutes(1));
    }
    let mut papers = handle.join().unwrap()?; //I do not like that we have to use unwrap here but I do not know how to use ? with error_chain with these types

    papers.append(&mut eccc_papers);
    info!("All papers we found but not filtered: {:?}", &papers);

    //let mut papers_filtered = types::filter_papers(papers, utc);
    //info!("Filtered papers: {:?}", &papers_filtered);
    progress_fetch_bar.set_message("Sorting and Deduping");
    papers.sort_unstable();
    types::dedup_papers(&mut papers);

    progress_fetch_bar.finish_with_message("Done fetching");

    Ok(papers)
}

fn setup() -> Result<(oauth2::basic::BasicTokenResponse, String)> {
    let tk = drive_oauth::setup_oauth()?;
    let directory_id = if let Some(id) = config::read_directory_id() {
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

    let config = Arc::new(RwLock::new(config::Config::read_or_default()));
    let tk = if config.read().unwrap().local_store.is_none() {
        Some(drive_oauth::setup_oauth()?)
    } else {
        None
    };

    let filtered_papers = get_and_filter_papers(&config)?;

    if filtered_papers.is_empty() {
        println!("Nothing new found. Saving new date.");
        //return config::write_now();
    }

    let is_filtered_empty = filtered_papers.is_empty();
    if let Some(papers_to_download) = gui::get_selected_papers(filtered_papers) {
        if papers_to_download.is_empty() && !is_filtered_empty {
            println!("No papers to download ({})", console::Emoji("✅", ""));
            return Ok(());
            //return config::write_paper_published(last_paper);
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
                let c = config.read().unwrap();
                drive::upload_file_or_local(&tk, f, i.paper, &c.directory_id, &c.local_store)
                    .context("Uploading function has error")?;
            }
            progressbar.finish();
            (*config.write().unwrap()).write()?;
            //config::write_paper_published(last_paper)?;
            //config::write_now()?;
        }
    } else {
        println!(
            "Nothing to download ({}) and we are not saving ({}).",
            console::Emoji("❌", ""),
            console::Emoji("❌", "")
        );
    }
    Ok(())
}

fn main() {
    pretty_env_logger::init();
    let matches = App::new("TheoryGrabber")
        .version(crate_version!())
        .author("Narfinger")
        .about("Grabs ArXiv and ECCC papers, lists them, downloads them and uploads them to a folder in Google Drive.")
        .arg(Arg::with_name("clean")
             .short("c")
             .long("clean")
             .help("Sets the current date as a date in the config. This is should only be used before the first download \
                    to not make you go through papers you already know."))
        .arg(Arg::with_name("local")
            .short("l")
            .long("local")
            .help("Sets the download to local")
            .takes_value(true))
        .arg(Arg::with_name("oauth")
            .short("o")
            .long("oauth")
            .help("Sets the config with oauth"))
        .get_matches();

    if matches.is_present("clean") {
        let res = setup();
        if res.is_err() {
            println!("Could not write file, something is wrong.");
        }
    } else if let Some(dir) = matches.value_of("local") {
        let mut c = config::Config::read_or_default();
        if !Path::new(dir).exists() {
            println!("Directory does not exist");
            return;
        }
        let path = Path::new(dir)
            .canonicalize()
            .expect("Cannot canonicalize the path");
        c.local_store = Some(dir);
        c.write().expect("Could not write config");
    } else if matches.is_present("oauth") {
        setup().expect("Error in setting up oauth");
    } else if config::Config::read().is_err() {
        println!("Please initialize config with the oauth or local option.");
        return;
    }

    run().expect("Error");
}
