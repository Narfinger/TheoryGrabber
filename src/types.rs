use chrono;
use chrono::Weekday;
use chrono_tz::America::New_York;
use chrono::Datelike;
use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;
use chrono::Duration;
use url;
#[cfg(test)]
use chrono::Utc;



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

#[test]
fn filter_test() {
    let exurl = url::Url::parse("https://www.example.com").unwrap();
    let p1 = Paper { title: String::from("Test1"), description: String::from("a"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author1"), String::from("Author2")], source: Source::ECCC};

    let now = Utc::now();
    
    let p2 = Paper {title: String::from("Test2"), description: String::from("b"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author2"), String::from("Author1")], source: Source::Arxiv};
    let p3 = Paper {title: String::from("Test3"), description: String::from("c"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author3"), String::from("Author2")], source: Source::Arxiv};

    let vec = vec![p1.clone(), p2.clone(), p3.clone()];
    assert_eq!(filter_papers(vec, now), vec![p2,p3]);
}

/// Special filter function because arxiv is a bit special.
/// Arxiv does
/// not publish on weekends.  For all other days, the following could
/// occur. Arxiv publishes at (for example) UTC 00:00. Papers that are
/// submitted at UTC 01:00 are not included. If we check at UTC 02:00,
/// we lose these papers as we filter with UTC>= 02:00.
fn special_filter(date: chrono::DateTime<chrono::Utc>, p: &Paper) -> bool {
    match p.source {
        Source::ECCC => p.published >= date,
        Source::Arxiv => {
            let mut perhaps_date: chrono::DateTime<chrono::Utc> = date.date().with_timezone(&New_York).and_hms(14,0,0).with_timezone(&chrono::Utc);
            if perhaps_date >= date {
                perhaps_date = perhaps_date - Duration::days(1);
            }
            if perhaps_date.weekday() == Weekday::Sat {
                perhaps_date = perhaps_date - Duration::days(1);
            } else if perhaps_date.weekday() == Weekday::Sun {
                perhaps_date = perhaps_date - Duration::days(2);
            }
                        
            p.published > perhaps_date
        }
    } 
}

/// Filters papers.
/// Arxiv needs a special filtering as there could be following occurence.
/// Arxiv publishes at (for example) UTC 00:00. Papers that are submitted  at UTC 01:00 are not included. If we check at UTC 02:00, we lose these papers as we filter with UTC>= 02:00. We use `special_filter`.
pub fn filter_papers(paper: Vec<Paper>, date: chrono::DateTime<chrono::Utc>) -> Vec<Paper> {
    paper
        .into_iter()
        .filter(|p| special_filter(date, p))
        .collect::<Vec<Paper>>()
}

#[test]
fn fuzzy_exists_test() {
    let exurl = url::Url::parse("https://www.example.com").unwrap();
    let p1 = Paper { title: String::from("Test1"), description: String::from("a"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author1"), String::from("Author2")], source: Source::ECCC};
    let p1eq = Paper {title: String::from("Test1"), description: String::from("b"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author2"), String::from("Author1")], source: Source::Arxiv};
    let p2 = Paper {title: String::from("Test1 t"), description: String::from("c"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author3"), String::from("Author2")], source: Source::Arxiv};

    let vec = vec![p1.clone(), p1eq.clone(), p2.clone()];
    assert_eq!(fuzzy_exists_equal_different_source(&vec, &p1), false);
    assert_eq!(fuzzy_exists_equal_different_source(&vec, &p1eq), true);
    assert_eq!(fuzzy_exists_equal_different_source(&vec, &p2), false);
}

/// If we find an eccc and an arxiv version,
fn fuzzy_exists_equal_different_source(v: &[Paper], p: &Paper) -> bool {
    v.iter().any(|q| p.source == Source::Arxiv && q.source == Source::ECCC && p.title == q.title)
}

#[test]
fn dedup_test() {
    let exurl = url::Url::parse("https://www.example.com").unwrap();
    let p1 = Paper { title: String::from("Test1"), description: String::from("a"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author1"), String::from("Author2")], source: Source::ECCC};
    let p1eq = Paper {title: String::from("Test1"), description: String::from("b"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author2"), String::from("Author1")], source: Source::Arxiv};
    let p2 = Paper {title: String::from("Test1 t"), description: String::from("c"), link: exurl.clone(), published: Utc::now(), authors: vec![String::from("Author3"), String::from("Author2")], source: Source::Arxiv};
    
    

    let mut v = vec![p1.clone(), p1eq, p2.clone()];
    dedup_papers(&mut v);
    assert_eq!(vec![p1.clone(),p2.clone()], v);
}

/// Remove papers that are doubled in the vector and keep the ECCC version. The test cases are probably the best example. 
pub fn dedup_papers(paper: &mut Vec<Paper>) {
    let v = paper.clone();
    paper.retain(|q| !fuzzy_exists_equal_different_source(&v, q));
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum BasicColumn {
    Title,
    Source,
    Published,
}
