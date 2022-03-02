#![recursion_limit = "128"]
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
extern crate console;
extern crate nom;
extern crate quick_xml;
extern crate tempfile;
extern crate tokio;
extern crate toml;
extern crate url;

pub mod arxiv;
pub mod config;
pub mod drive;
pub mod eccc;
pub mod gui;
pub mod paper_dialog;
pub mod types;

use crate::types::{DownloadedPaper, Paper};
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::copy;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use tempfile::TempDir;

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
            .unwrap_or_else(chrono::Utc::now);
        let arxiv_papers = arxiv::parse_arxiv().map(|p| types::filter_papers(p, time));
        if let Ok(mut arxiv_papers) = arxiv_papers {
            arxiv_papers.sort_unstable();
            if let Some(p) = arxiv_papers.last() {
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
        .unwrap_or_else(chrono::Utc::now);
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
    types::remove_downloaded(&mut papers, config);

    progress_fetch_bar.finish_with_message("Done fetching");

    Ok(papers)
}

fn setup() -> Result<String> {
    //let tk = drive_oauth::setup_oauth()?;
    let directory_id = if let Some(id) = config::read_directory_id() {
        id
    } else {
        //let id = drive::create_directory(&tk)?;
        //config::write_directory_id_and_now(id.clone())?;
        "NULL".to_string()
    };

    Ok(directory_id)
}

fn run() -> Result<()> {
    println!("Ideas for improvement:");
    println!("sub implement dialog for deleting to support delete key and stuff");
    println!("better error handling for resumeable downloads\n");

    let config = Arc::new(RwLock::new(config::Config::read_or_default()));
    let tk: Option<String> = None;

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

        if let Ok(dir) = TempDir::new() {
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
                drive::upload_file_or_local(f, i.paper, &c.directory_id, &c.local_store)
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
            console::Emoji("❌", "X"),
            console::Emoji("❌", "X")
        );
    }
    Ok(())
}

///Grabs papers from arxiv and eccc and puts them into a local directory
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets the current date as a date in the config. This should be used before the first download.
    #[clap(short, long)]
    clean: bool,

    /// Sets the download to local with this as a directory
    #[clap(short, long)]
    local: Option<String>,
}

fn main() {
    pretty_env_logger::init();
    let args = Args::parse();

    if args.clean {
        let res = setup();
        if res.is_err() {
            println!("Could not write file, something is wrong.");
        }
    } else if let Some(dir) = args.local {
        let mut c = config::Config::read_or_default();
        if !Path::new(&dir).exists() {
            println!("Directory does not exist");
            return;
        }
        let path = Path::new(&dir)
            .canonicalize()
            .expect("Cannot canonicalize the path");
        c.local_store = Some(String::from(path.to_str().unwrap()));
        c.write().expect("Could not write config");
    } else if args.oauth {
        println!("Warning, Oauth is currently very flaky and might not work");
        setup().expect("Error in setting up oauth");
    } else if config::Config::read().is_err() {
        println!("Please initialize config with the oauth or local option.");
        return;
    }

    run().expect("Error");
}
