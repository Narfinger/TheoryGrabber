use std::io::Read;
use reqwest;
use std;
use quick_xml;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use url::Url;
use types::{Paper, Source};


static ARXIV: &'static str = "http://export.arxiv.org/api/query?search_query=cat:cs.CC&sortBy=lastUpdatedDate&sortOrder=descending&max_results=10";

pub fn parse_arxiv() -> Vec<Paper> {
    enum Tag {
        Entry,
        Published,
        Author,
        Title,
        Summary,
        Nothing,
    };
    #[derive(Debug)]
    struct TmpPaper {
        published: Option<String>,
        title: Option<String>,
        summary: Option<String>,
        link: Option<String>,
        authors: Vec<String>,
    };


    //this is super inefficient!
    let mut reqreader = reqwest::get(ARXIV).unwrap();
    let mut resp = String::new();
    reqreader.read_to_string(&mut resp);
    println!("{}", resp);


    let mut reader = Reader::from_str(resp.as_str());
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
        match reader.read_event(&mut buf) {
            Ok(Event::End(ref e)) => {
                match e.name() {
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
                }
            }
            Ok(Event::Start(ref e)) => {
                match e.name() {
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
                        println!("STSDLKJFS");
                        let mut bl: quick_xml::events::attributes::Attributes = e.attributes();
                        let it: &[u8] = bl.find(|i| i.as_ref().unwrap().key == b"href")
                            .unwrap()
                            .unwrap()
                            .value;
                        //let itowned: Vec<u8> = it.to_owned();
                        let res: &str = std::str::from_utf8(it).unwrap();
                        let ress = res.to_string().clone();
                        cur_paper.link = Some(ress);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                if let b"link" = e.name() {
                    let mut bl: quick_xml::events::attributes::Attributes = e.attributes();
                    let it: &[u8] = bl.find(|i| i.as_ref().unwrap().key == b"href")
                        .unwrap()
                        .unwrap()
                        .value;
                    //let itowned: Vec<u8> = it.to_owned();
                    let res: &str = std::str::from_utf8(it).unwrap();
                    let ress = res.to_string().clone();
                    cur_paper.link = Some(ress);
                }
            }
            Ok(Event::Text(e)) => {
                match tag {
                    Tag::Published => {
                        cur_paper.published = Some(e.unescape_and_decode(&reader).unwrap())
                    } 
                    Tag::Title => cur_paper.title = Some(e.unescape_and_decode(&reader).unwrap()), 
                    Tag::Summary => {
                        cur_paper.summary = Some(e.unescape_and_decode(&reader).unwrap())
                    }
                    Tag::Author => {
                        cur_paper
                            .authors
                            .push(e.unescape_and_decode(&reader).unwrap())
                    }
                    //Tag::Link => cur_paper.link = Some("http://localhost".to_string()), //Some(e.unescape_and_decode(&reader).unwrap()),
                    Tag::Nothing | Tag::Entry => {}  
                }


                e.unescape_and_decode(&reader).unwrap();
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
    }


    println!("PARSING DONE");
    //converting tmp to real and testing
    entries
        .into_iter()
        .map(|p| {
            //println!("{:?}", p);
            Paper {
                title: p.title.unwrap(),
                description: p.summary.unwrap(),
                published: 0, /* p.published */
                link: Url::parse(p.link.unwrap().as_str()).unwrap(),
                source: Source::Arxiv,
                authors: p.authors,
            }
        })
        .collect::<Vec<Paper>>()
}
