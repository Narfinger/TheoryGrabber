use crate::errors::*;
use crate::types::Paper;
use nom::character::complete::digit1;
use oauth2;
use reqwest;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_RANGE, LOCATION};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str;

static UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable";
static DIRECTORY_URL: &str = "https://www.googleapis.com/drive/v3/files";
static DIRECTORY_NAME: &str = "TheoryGrabber";

/// The response to a file create query.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileCreateResponse {
    kind: String,
    id: String,
    name: String,
    mime_type: String,
}

/// Json for uploading a file.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileUploadJSON {
    name: String,
    mime_type: String,
    parents: Vec<String>,
}

/// Returns the initial of the last name of an author.
fn get_last_name_initials(author: &str) -> char {
    let lastname = author
        .split_whitespace()
        .nth(1)
        .expect("No lastname found?"); //lastname
    lastname.chars().next().unwrap()
}

/// Returns the author string we will use. Uses `get_last_name_initials`.
fn author_string(paper: &Paper) -> String {
    paper
        .authors
        .iter()
        .fold(String::from(""), |acc, i| {
            acc + &get_last_name_initials(i).to_string()
        })
        .to_uppercase()
}

/// Returns the filename we will save as for a given filename.
fn make_filename(paper: &Paper) -> String {
    let datestring = paper.published.format("%Y-%m-%d");
    let mut title = paper.title.to_owned();
    title.truncate(75);

    datestring.to_string() + "-" + &author_string(paper) + "-" + &title + ".pdf"
}

/// Creates directory in google drive. If called multiple times, will create multiple directories and saves the last directory id to the configuration file.
pub fn create_directory(tk: &oauth2::Token) -> Result<String> {
    let client = reqwest::Client::new();
    let mut header = HeaderMap::new();

    header.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&tk.access_token).unwrap(),
    );
    let mut metadata = HashMap::new();
    metadata.insert("name", DIRECTORY_NAME);
    metadata.insert("mimeType", "application/vnd.google-apps.folder");

    let mut res = client
        .post(DIRECTORY_URL)
        .headers(header.clone())
        .json(&metadata)
        .send()
        .chain_err(|| "Error in sending to create directory")?;

    let response: FileCreateResponse = res.json().chain_err(|| "Error in decoding Response")?;

    Ok(response.id)
}

struct ContentRange {
    from: u32,
    to: u32,
}

named!(
    number<u32>,
    map_res!(map_res!(digit1, str::from_utf8), |s: &str| s.parse::<u32>())
);

named!(content_range<&[u8], ContentRange>,
    do_parse!(
        v: number >>
        ws!(tag!("-")) >>
        w: number >>
        (ContentRange { from: v, to: w }))
);

fn parse_content_range(range: &str) -> Result<ContentRange> {
    if let Ok((_, l)) = content_range(range.as_bytes()) {
        Ok(l)
    } else {
        Err(format!("invalid content_range, str was {}", range).into())
    }
}

/// Tries to resume an upload if an error happened
/// gets `id` which is the file id, `loc` which is the resumeable url and `f` which is the file
/// See: <https://developers.google.com/drive/v3/web/resumable-upload#resume-upload>
fn resume_upload(loc: &str, mut f: File, h: &HeaderMap) -> Result<()> {
    println!("Starting resume upload");
    let client = reqwest::Client::new();
    let mut header = h.clone();
    header.insert(CONTENT_RANGE, HeaderValue::from_static("*-*"));
    let res = client.put(loc).send()?;
    println!("Send put request");
    if (res.status() == reqwest::StatusCode::OK) | (res.status() == reqwest::StatusCode::CREATED) {
        Ok(())
    } else if res.status() == reqwest::StatusCode::NOT_FOUND {
        Err("Upload url not found, something is wrong".into())
    } else if res.status() == reqwest::StatusCode::PERMANENT_REDIRECT {
        println!("Getting correct status code");
        if let Some(ct) = res.headers().get(CONTENT_RANGE) {
            println!("Getting target range");
            let content_range = parse_content_range(ct.to_str().unwrap());
            if let Ok(c) = content_range {
                println!("Seeking the file back");
                f.seek(SeekFrom::Start(0))?;
                println!("Getting slices");
                let mut slices = vec![0u8; (c.to as usize) - (c.from as usize)];
                f.read_exact(&mut slices)?;
                println!("Sending upload request");
                let res = client
                    .put(&loc.to_string())
                    .headers(h.clone())
                    .body(slices)
                    .send();
                println!("Upload request sent");
                if res.is_ok() {
                    Ok(())
                } else {
                    Err("We tried one resume and we could not finish".into())
                }
            } else {
                Err("content range spec could not work".into())
            }
        } else {
            Err("Could not find Content Range header".into())
        }
    } else {
        Err("Unknown response returned".into())
    }
}

/// Uploads a file to google drive to the directory given by `fileid`.
/// This uses the resubmeable upload feature by first uploading the metadata and then uploading the file via the resumeable url method.
/// Currently we do not support resuming a broken upload and just error out.
pub fn upload_file(tk: &oauth2::Token, f: File, paper: &Paper, fileid: &str) -> Result<()> {
    //getting the proper resumeable session URL
    let client = reqwest::Client::new();
    let mut header = HeaderMap::new();

    let authstring = "Bearer ".to_owned() + &tk.access_token;
    header.insert(AUTHORIZATION, HeaderValue::from_str(&authstring).unwrap());

    let filename = make_filename(paper);

    let metadata = FileUploadJSON {
        name: filename,
        mime_type: "application/pdf".to_string(),
        parents: vec![String::from(fileid)],
    };

    let query = client
        .post(UPLOAD_URL)
        .headers(header.clone())
        .json(&metadata)
        .build();

    let res = client
        .execute(query.unwrap())
        .chain_err(|| "Error in getting resumeable url")?;

    if res.status().is_success() {
        if let Some(loc) = res.headers().get(LOCATION) {
            let fclone = f.try_clone().unwrap();
            let upload_res = client
                .put(loc.to_str().unwrap())
                .headers(header.clone())
                .body(f)
                .send();
            if upload_res.is_ok() {
                Ok(())
            } else {
                resume_upload(loc.to_str().unwrap(), fclone, &header)
            }
        } else {
            Err("no location header found".into())
        }
    } else {
        Err("Something went wrong with getting resumeable url".into())
    }
}
