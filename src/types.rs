use crate::config::Config;
use anyhow::Result;
use chrono::Datelike;
use chrono::Duration;
use chrono::Timelike;
#[cfg(test)]
use chrono::Utc;
use chrono::Weekday;
use chrono_tz::America::New_York;
use directories::ProjectDirs;
use itertools::Itertools;
use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;

pub(crate) fn get_config_dir() -> Result<std::path::PathBuf> {
    ProjectDirs::from("com", "narfinger", "TheoryGrabber")
        .map(|p| p.config_dir().to_path_buf())
        .ok_or_else(|| anyhow!("Error in getting config file"))
}

/// Shows where the paper came from.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum Source {
    /// The Source is ArXiV.org.
    Arxiv(String),
    /// The source is ECCC (Electronic Colloqium on Complexity).
    ECCC,
}

impl Source {
    pub(crate) fn is_arxiv(&self) -> bool {
        matches!(self, Source::Arxiv(_))
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out = match self {
            Source::Arxiv(s) => format!("ArXiV({})", &s),
            Source::ECCC => String::from("ECCC"),
        };

        write!(f, "{out}")
    }
}

/// Struct for denoting a paper.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct Paper {
    /// The title of the paper.
    pub(crate) title: String,
    /// Description/Abstract of the paper.
    pub(crate) description: String,
    /// Url where the paper can be downloaded.
    pub(crate) link: reqwest::Url,
    /// Where did we parse the paper from.
    pub(crate) source: Source,
    /// The publication date as given by the source in Utc timezone.
    pub(crate) published: chrono::DateTime<chrono::Utc>,
    /// A list of authors which can be in arbitrary order.
    pub(crate) authors: Vec<String>,
}

impl std::fmt::Display for Paper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl std::fmt::Debug for Paper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.source, self.title, self.published)
    }
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

impl Paper {
    /// Returns the author string we will use for filename. Uses `get_last_name_initials`.
    pub(crate) fn author_string(&self) -> String {
        self.authors
            .iter()
            .fold(String::from(""), |acc, i| {
                let lastname = {
                    let lastname = i.split_whitespace().nth(1).unwrap_or("No lastname"); //lastname
                    lastname.chars().next().unwrap()
                };

                acc + &lastname.to_string()
            })
            .to_uppercase()
    }

    pub(crate) fn format_author(&self) -> String {
        Itertools::intersperse(
            self.authors
                .iter()
                .map(|a| a.split_whitespace().last().unwrap_or("")),
            ", ",
        )
        .collect()
    }

    /// Returns the filename we will save as for a given filename.
    pub(crate) fn filename(&self) -> String {
        let datestring = self.published.format("%Y-%m-%d");
        let mut title = self.title.to_owned();
        title.truncate(75);

        // we do not want zeroes
        datestring.to_string() + "-" + &self.author_string() + "-" + title.trim_end() + ".pdf"
    }
}

/// This represents a paper that is downloaded.
pub(crate) struct DownloadedPaper<'a> {
    /// The paper that is downloaded
    pub(crate) paper: &'a Paper,
    /// The path where we stored the pdf file
    pub(crate) path: PathBuf,
}

#[test]
fn filter_test() {
    let exurl = reqwest::Url::parse("https://www.example.com").unwrap();
    let p1 = Paper {
        title: String::from("Test1"),
        description: String::from("a"),
        link: exurl.clone(),
        published: Utc::now(),
        authors: vec![String::from("Author1"), String::from("Author2")],
        source: Source::ECCC,
    };

    let now = Utc::now();

    let p2 = Paper {
        title: String::from("Test2"),
        description: String::from("b"),
        link: exurl.clone(),
        published: Utc::now(),
        authors: vec![String::from("Author2"), String::from("Author1")],
        source: Source::Arxiv(String::from("")),
    };
    let p3 = Paper {
        title: String::from("Test3"),
        description: String::from("c"),
        link: exurl,
        published: Utc::now(),
        authors: vec![String::from("Author3"), String::from("Author2")],
        source: Source::Arxiv(String::from("")),
    };

    let vec = vec![p1, p2.clone(), p3.clone()];
    assert_eq!(filter_papers(vec, now), vec![p2, p3]);
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
        Source::Arxiv(_) => {
            let mut perhaps_date: chrono::DateTime<chrono::Utc> = date
                .with_timezone(&New_York)
                .with_hour(14)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_timezone(&chrono::Utc);
            if perhaps_date >= date {
                perhaps_date -= Duration::days(1);
            }
            if perhaps_date.weekday() == Weekday::Sat {
                perhaps_date -= Duration::days(1);
            } else if perhaps_date.weekday() == Weekday::Sun {
                perhaps_date -= Duration::days(2);
            }

            p.published > perhaps_date
        }
    }
}

/// Filters papers.
/// Arxiv needs a special filtering as there could be following occurence.
/// Arxiv publishes at (for example) UTC 00:00. Papers that are submitted  at UTC 01:00 are not included. If we check at UTC 02:00, we lose these papers as we filter with UTC>= 02:00. We use `special_filter`.
pub(crate) fn filter_papers(paper: Vec<Paper>, date: chrono::DateTime<chrono::Utc>) -> Vec<Paper> {
    paper
        .into_iter()
        .filter(|p| special_filter(date, p))
        .collect::<Vec<Paper>>()
}

#[test]
fn fuzzy_exists_test() {
    let exurl = reqwest::Url::parse("https://www.example.com").unwrap();
    let p1 = Paper {
        title: String::from("Test1"),
        description: String::from("a"),
        link: exurl.clone(),
        published: Utc::now(),
        authors: vec![String::from("Author1"), String::from("Author2")],
        source: Source::ECCC,
    };
    let p1eq = Paper {
        title: String::from("Test1"),
        description: String::from("b"),
        link: exurl.clone(),
        published: Utc::now(),
        authors: vec![String::from("Author2"), String::from("Author1")],
        source: Source::Arxiv(String::from("")),
    };
    let p2 = Paper {
        title: String::from("Test1 t"),
        description: String::from("c"),
        link: exurl,
        published: Utc::now(),
        authors: vec![String::from("Author3"), String::from("Author2")],
        source: Source::Arxiv(String::from("")),
    };

    let vec = vec![p1.clone(), p1eq.clone(), p2.clone()];
    assert!(!fuzzy_exists_equal_different_source(&vec, &p1));
    assert!(fuzzy_exists_equal_different_source(&vec, &p1eq));
    assert!(!fuzzy_exists_equal_different_source(&vec, &p2));
}

/// If 'p' is an Arxiv version, test if there exists an ECCC version with the same title.
fn fuzzy_exists_equal_different_source(v: &[Paper], p: &Paper) -> bool {
    v.iter().any(|q| {
        (p.source.is_arxiv() && q.source == Source::ECCC && p.title == q.title)
            | (p.source.is_arxiv() && q.source.is_arxiv() && p.title == q.title)
    })
}

#[test]
fn dedup_test() {
    let exurl = reqwest::Url::parse("https://www.example.com").unwrap();
    let p1 = Paper {
        title: String::from("Test1"),
        description: String::from("a"),
        link: exurl.clone(),
        published: Utc::now(),
        authors: vec![String::from("Author1"), String::from("Author2")],
        source: Source::ECCC,
    };
    let p1eq = Paper {
        title: String::from("Test1"),
        description: String::from("b"),
        link: exurl.clone(),
        published: Utc::now(),
        authors: vec![String::from("Author2"), String::from("Author1")],
        source: Source::Arxiv(String::from("")),
    };
    let p2 = Paper {
        title: String::from("Test1 t"),
        description: String::from("c"),
        link: exurl,
        published: Utc::now(),
        authors: vec![String::from("Author3"), String::from("Author2")],
        source: Source::Arxiv(String::from("")),
    };

    let mut v = vec![p1.clone(), p1eq, p2.clone()];
    dedup_papers(&mut v);
    assert_eq!(vec![p1, p2], v);
}

/// Remove papers that are doubled in the vector and keep the ECCC version. The test cases are probably the best example.
pub(crate) fn dedup_papers(paper: &mut Vec<Paper>) {
    let v = paper.clone();
    paper.retain(|q| !fuzzy_exists_equal_different_source(&v, q));
}

/// do not show paper we already downloaded
pub(crate) fn remove_downloaded(paper: &mut Vec<Paper>, c: &Config) {
    if let Some(ref d) = c.local_store {
        let base_path = PathBuf::from(d);
        paper.retain(|p| !base_path.join(p.filename()).exists());
    }
}

pub(crate) fn sanitize_title(title: &str) -> String {
    title
        .replace('\n', "")
        .replace("  ", " ")
        .replace(std::path::MAIN_SEPARATOR, "")
}
