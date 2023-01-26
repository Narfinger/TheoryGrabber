use crate::types::{Paper, Source};
use anyhow::Result;
use log::info;
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use std::io::Read;
use url::Url;

static ARXIV: &str = "https://export.arxiv.org/api/query?search_query=cat:cs.CC&sortBy=lastUpdatedDate&sortOrder=descending&max_results=100";

/// Parses the Arxiv xml and gives the last 100 papers.
pub fn parse_arxiv() -> Result<Vec<Paper>> {
    enum Tag {
        Entry,
        Published,
        Author,
        Title,
        Summary,
        Nothing,
    }
    #[derive(Debug)]
    struct TmpPaper {
        published: Option<String>,
        title: Option<String>,
        summary: Option<String>,
        link: Option<String>,
        authors: Vec<String>,
    }

    //this is super inefficient!
    let client = reqwest::blocking::Client::builder()
        .user_agent("TheoryGrabber")
        .build()?;
    let mut reqreader = client.get(ARXIV).send()?;
    let mut resp = String::new();
    reqreader.read_to_string(&mut resp)?;

    let mut reader = Reader::from_str(&resp);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut tag = Tag::Nothing;
    let mut entries: Vec<TmpPaper> = Vec::new();
    let mut cur_paper: TmpPaper = TmpPaper {
        published: None,
        title: None,
        summary: None,
        link: None,
        authors: Vec::new(),
    };
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::End(ref e)) => match e.name().local_name().as_ref() {
                b"entry" => {
                    tag = Tag::Nothing;
                    entries.push(cur_paper);
                    cur_paper = TmpPaper {
                        published: None,
                        title: None,
                        summary: None,
                        link: None,
                        authors: Vec::new(),
                    };
                }
                _ => {
                    tag = Tag::Entry;
                }
            },
            Ok(Event::Start(ref e)) => match e.name().local_name().as_ref() {
                b"entry" => {
                    tag = Tag::Entry;
                    cur_paper = TmpPaper {
                        published: None,
                        title: None,
                        summary: None,
                        link: None,
                        authors: Vec::new(),
                    };
                }
                b"published" => {
                    tag = Tag::Published;
                }
                b"title" => {
                    tag = Tag::Title;
                }
                b"summary" => {
                    tag = Tag::Summary;
                }
                b"author" => {
                    tag = Tag::Author;
                }
                b"link" => {
                    let mut bl: quick_xml::events::attributes::Attributes = e.attributes();
                    let it = bl
                        .find(|i| i.as_ref().unwrap().key == QName(b"href"))
                        .unwrap()
                        .unwrap()
                        .value;
                    let res: &str = std::str::from_utf8(&it).expect("Error getting string");
                    let mut url = Url::parse(res).expect("Error parsing url");
                    url.set_host(Some("export.arxiv.org"))
                        .expect("Could not replace hostname");
                    cur_paper.link = Some(url.to_string());
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => {
                if let b"link" = e.name().local_name().as_ref() {
                    let mut bl: quick_xml::events::attributes::Attributes = e.attributes();
                    let it = bl
                        .find(|i| i.as_ref().unwrap().key == QName(b"href"))
                        .unwrap()
                        .unwrap()
                        .value;
                    let res: &str = std::str::from_utf8(&it).expect("Error getting string");
                    let mut url = Url::parse(res).expect("Error parsing url");
                    url.set_host(Some("export.arxiv.org"))
                        .expect("Could not replace hostname");
                    cur_paper.link = Some(url.to_string());
                }
            }
            Ok(Event::Text(e)) => {
                match tag {
                    Tag::Published => cur_paper.published = Some(e.unescape().unwrap().to_string()),
                    Tag::Title => cur_paper.title = Some(e.unescape().unwrap().to_string()),
                    Tag::Summary => cur_paper.summary = Some(e.unescape().unwrap().to_string()),
                    Tag::Author => cur_paper.authors.push(e.unescape().unwrap().to_string()),
                    //Tag::Link => cur_paper.link = Some("http://localhost".to_string()), //Some(e.unescape_and_decode(&reader).unwrap()),
                    Tag::Nothing | Tag::Entry => {}
                }

                e.unescape().unwrap().to_string();
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
    }

    info!("Found arxiv papers: {:?}", &entries);
    //converting tmp to real and testing
    Ok(entries
        .into_iter()
        .map(|p| {
            //println!("{:?}", p);
            Paper {
                title: crate::types::sanitize_title(&p.title.unwrap()),
                description: p.summary.unwrap(),
                published: chrono::DateTime::parse_from_rfc3339(&p.published.unwrap())
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                link: reqwest::Url::parse(&p.link.unwrap()).unwrap(),
                source: Source::Arxiv,
                authors: p.authors,
            }
        })
        .collect::<Vec<Paper>>())
}
#[test]
fn get_arxiv_papers_test() {
    assert!(parse_arxiv().is_ok());
}
