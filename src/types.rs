use chrono;
use std::fmt;
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

#[derive(Clone, Debug)]
pub struct Paper {
    pub title: String,
    pub description: String,
    pub link: url::Url,
    pub source: Source,
    pub published: chrono::DateTime<chrono::Utc>,
    pub authors: Vec<String>,
}

pub fn print_authors(paper: &Paper) -> String {
    //if let Some(x) = paper.authors.first() {
    paper
        .authors
        .iter()
        .fold("".to_string(), |acc, x| acc + " and " + x.as_str())
    //} else {
    //    "".to_string()
    //}
}


pub fn filter_papers(paper: Vec<Paper>, date: chrono::DateTime<chrono::Utc>) -> Vec<Paper> {
    paper
        .into_iter()
        .filter(|p| p.published > date)
        .collect::<Vec<Paper>>()
}
