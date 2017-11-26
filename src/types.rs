use chrono;
use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;
use url;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Source {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Paper {
    pub title: String,
    pub description: String,
    pub link: url::Url,
    pub source: Source,
    pub published: chrono::DateTime<chrono::Utc>,
    pub authors: Vec<String>,
}

impl Ord for Paper {
    fn cmp(&self, other: &Paper) -> Ordering {
        self.published.cmp(&other.published)
    }
}

impl PartialOrd for Paper {
    fn partial_cmp(&self, other: &Paper) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct DownloadedPaper<'a> {
    pub paper: &'a Paper,
    pub path: PathBuf,
}

pub fn print_authors(paper: &Paper) -> String {
    //if let Some(x) = paper.authors.first() {
    let mut iterator = paper.authors.iter();
    let first = iterator.next().unwrap();
    iterator.fold(first.to_string(), |acc, x| acc + " and " + x)
    //} else {
    //    "".to_string()
    //}
}


pub fn filter_papers(paper: Vec<Paper>, date: chrono::DateTime<chrono::Utc>) -> Vec<Paper> {
    paper
        .into_iter()
        .filter(|p| p.published >= date)
        .collect::<Vec<Paper>>()
}

fn fuzzy_is_equal_different_source(v: &Vec<Paper>, p: &Paper) -> bool {
    !v.iter().any(|q| p.source!=q.source && p.title==q.title)
}

pub fn dedup_papers(paper: &mut Vec<Paper>) {
    let v = paper.clone();
    paper.retain(|q| !fuzzy_is_equal_different_source(&v, q));
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum BasicColumn {
    Title,
    Source,
    Published,
}
