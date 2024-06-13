#[macro_use]
extern crate anyhow;

pub mod arxiv;
pub mod config;
pub mod drive;
pub mod eccc;
pub mod gui;
pub mod types;

use crate::types::{DownloadedPaper, Paper};
use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use gui::SelectedPapers;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::info;
use reqwest::blocking::Client;
use std::fs::File;
use std::io::copy;
use std::path::{Path, PathBuf};
use std::thread;
use tempfile::TempDir;
use types::{NewDate, Source};

const OK_EMOJI: char = '✅';
const NOT_OK_EMOJI: char = '❌';
const THUMPSUP_EMOJI: char = '👍';

/// Downloads a single paper and returns the path it was aved to
fn download_paper<'a>(client: &'a Client, dir: &TempDir, paper: &'a Paper) -> Result<PathBuf> {
    let mut response = client.get(paper.link.clone()).send()?;
    let contenttype = response.headers().get(reqwest::header::CONTENT_TYPE);
    if contenttype
        != Some(
            &reqwest::header::HeaderValue::from_str("application/pdf")
                .expect("Error in parsing header"),
        )
    {
        Err(anyhow!("Response invalid"))?
    } else {
        let filename = Path::new("")
            .with_file_name(&paper.title)
            .with_extension("pdf");
        let savefile = dir.path().join(&filename);
        let mut file = File::create(savefile.clone()).context(anyhow!(
            "Error in file with filename: {:?}, savefile: {:?}",
            filename.clone(),
            savefile.clone()
        ))?;
        info!("Downloading {} onto {:?}", &paper.title, &savefile);
        copy(&mut response, &mut file).context("Downloading paper")?;
        Ok(savefile)
    }
}
/// Downloads the papers to the `TempDir`.
fn download_papers<'a>(
    papers: &'a [Paper],
    dir: &TempDir,
    args: &Args,
) -> Result<Vec<DownloadedPaper<'a>>> {
    if !papers.is_empty() {
        let mut files: Vec<DownloadedPaper> = Vec::new();
        let mut errored_out: Vec<Paper> = Vec::new();
        let progressbar = ProgressBar::new(papers.len() as u64);
        progressbar.set_message("Downloading Papers");
        progressbar.set_style(
        ProgressStyle::default_bar()
        .template(
            "[{elapsed_precise}] {msg} {spinner:.green} {bar:100.green/blue} {pos:>7}/{len:7}",
        )?
        .progress_chars("#>-"),
    );
        progressbar.enable_steady_tick(std::time::Duration::new(0, 100));
        if args.quiet {
            progressbar.set_draw_target(ProgressDrawTarget::hidden())
        }

        let client = reqwest::blocking::Client::builder()
            .user_agent("TheoryGrabber")
            .build()?;
        for i in progressbar.wrap_iter(papers.iter()) {
            if let Ok(path) = download_paper(&client, dir, i) {
                files.push(DownloadedPaper { paper: i, path });
            } else {
                errored_out.push(i.clone());
            }
        }
        progressbar.finish();

        if !errored_out.is_empty() {
            println!("\nCould not download the following papers:");
            for i in errored_out {
                println!("{}", i.link);
            }
        }

        Ok(files)
    } else {
        Ok(vec![])
    }
}

/// Fetches the papers and filters for new papers.
fn get_and_filter_papers(config: &config::Config, args: &Args) -> Result<Vec<Paper>> {
    let progress_fetch_bar = ProgressBar::new_spinner();
    progress_fetch_bar.set_message("Getting Arxiv/ECCC");
    progress_fetch_bar.set_style(
        ProgressStyle::default_bar().template("[{elapsed_precise}] {msg} {spinner:.green}")?,
    );
    progress_fetch_bar.enable_steady_tick(std::time::Duration::new(0, 100));
    if args.quiet {
        progress_fetch_bar.set_draw_target(ProgressDrawTarget::hidden())
    }

    let time = config.last_checked_arxiv.unwrap_or_else(chrono::Utc::now);
    let handle = thread::spawn(move || {
        let arxiv_papers = arxiv::parse_arxiv().map(|p| types::filter_papers(p, time));
        if let Ok(mut arxiv_papers) = arxiv_papers {
            arxiv_papers.sort_unstable();
            Ok(arxiv_papers)
        } else {
            arxiv_papers
        }
    });
    let eccc_time = config.last_checked_eccc.unwrap_or_else(chrono::Utc::now);
    let eccc_papers = eccc::parse_eccc(eccc_time).context("Error in parsing eccc")?;
    let mut eccc_papers = types::filter_papers(eccc_papers, eccc_time);
    eccc_papers.sort_unstable();
    let mut papers = handle.join().unwrap()?; //I do not like that we have to use unwrap here but I do not know how to use ? with error_chain with these types

    papers.append(&mut eccc_papers);
    info!("All papers we found but not filtered: {:?}", &papers);

    //let mut papers_filtered = types::filter_papers(papers, utc);
    //info!("Filtered papers: {:?}", &papers_filtered);
    progress_fetch_bar.set_message("Sorting and Deduping");
    papers.sort_unstable();
    papers = types::dedup_papers(papers);
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

/// downloads and puts an array of papers into their folders
fn handle_papers(papers: &[Paper], config: &Config, args: &Args) -> Result<()> {
    let dir = TempDir::new()?;
    let files = download_papers(papers, &dir, args).context("Files error")?;

    let progressbar = ProgressBar::new(files.len() as u64);
    progressbar.set_message("Uploading Papers");
    progressbar.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed_precise}] {msg} {spinner:.green} {bar:100.green/blue} {pos:>7}/{len:7}",
            )?
            .progress_chars("#>-"),
    );
    progressbar.enable_steady_tick(std::time::Duration::new(0, 100));
    if args.quiet {
        progressbar.set_draw_target(ProgressDrawTarget::hidden())
    }

    for i in progressbar.wrap_iter(files.iter()) {
        let f = File::open(i.path.clone()).context("File couldn't be opened")?;
        drive::upload_file_or_local(f, i.paper, &config.directory_id, &config.local_store)
            .context("Uploading function has error")?;
    }
    progressbar.finish();
    Ok(())
}

fn get_latest_dates(papers: &[Paper]) -> NewDate {
    let new_arxiv_date = papers
        .iter()
        .filter(|p| p.source.is_arxiv())
        .map(|p| p.published)
        .next();
    let new_eccc_date = papers
        .iter()
        .filter(|p| p.source == Source::ECCC)
        .map(|p| p.published)
        .next();

    NewDate {
        eccc: new_eccc_date,
        arxiv: new_arxiv_date,
    }
}

fn run(args: &Args) -> Result<()> {
    let mut config = config::Config::read_or_default();
    let mut filtered_papers = get_and_filter_papers(&config, args)?;

    filtered_papers.sort_unstable_by(|a, b| b.cmp(a));
    // we need to get the first one as it is in descending order
    let new_dates = get_latest_dates(&filtered_papers);
    let new_arxiv_date = new_dates.arxiv.or(config.last_checked_arxiv);
    let new_eccc_date = new_dates.eccc.or(config.last_checked_eccc);
    let filter_date = config.last_checked_arxiv.unwrap();
    config.last_checked_arxiv = new_arxiv_date;
    config.last_checked_eccc = new_eccc_date;

    match gui::get_selected_papers(filtered_papers, filter_date)? {
        SelectedPapers::Abort => {
            println!(
                "Nothing to download ({}) and we are not saving ({}).",
                console::Emoji(&NOT_OK_EMOJI.to_string(), "X"),
                console::Emoji(&NOT_OK_EMOJI.to_string(), "X")
            );
            Ok(())
        }
        SelectedPapers::NoNew => {
            println!(
                "No papers to download ({}) we are saving ({})",
                console::Emoji(&OK_EMOJI.to_string(), ""),
                console::Emoji(&OK_EMOJI.to_string(), "")
            );
            println!("Config: {:?}", &config);
            config.write()
        }
        SelectedPapers::Selected(papers) => {
            handle_papers(&papers, &config, args)?;
            let c = config.write();
            println!(
                "We downloaded {} papers {}",
                papers.len(),
                console::Emoji(&THUMPSUP_EMOJI.to_string(), "")
            );
            c
        }

        SelectedPapers::SelectedAndSavedAt(papers, new) => {
            handle_papers(&papers, &config, args)?;
            // now we need to modify the date
            if config.last_checked_arxiv.unwrap() <= new.arxiv.unwrap_or_default() {
                config.last_checked_arxiv = new.arxiv;
            }
            if config.last_checked_eccc.unwrap() <= new.eccc.unwrap_or_default() {
                config.last_checked_eccc = new.eccc;
            }
            println!(
                "Downloading {} and saving at {} and {}",
                papers.len(),
                config.last_checked_arxiv.unwrap_or_default(),
                config.last_checked_eccc.unwrap_or_default()
            );
            config.write()
        }
    }
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

    /// hide progress bar
    #[clap(short, long)]
    quiet: bool,
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
    } else if config::Config::read().is_err() {
        println!("Please initialize config with the oauth or local option.");
        return;
    } else {
        run(&args).expect("Error");
    }
}
