use crate::oauth2::TokenResponse;
use crate::types::Paper;
use anyhow::{Context, Result};
use nom::character::complete::digit1;
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
pub fn create_directory(tk: &oauth2::basic::BasicTokenResponse) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let mut header = HeaderMap::new();

    header.insert(
        AUTHORIZATION,
        HeaderValue::from_str(tk.access_token().secret()).unwrap(),
    );
    let mut metadata = HashMap::new();
    metadata.insert("name", DIRECTORY_NAME);
    metadata.insert("mimeType", "application/vnd.google-apps.folder");

    let res = client
        .post(DIRECTORY_URL)
        .headers(header)
        .json(&metadata)
        .send()
        .context("Error in sending to create directory")?;

    let response: FileCreateResponse = res.json().context("Error in decoding Response")?;

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
        Err(anyhow!("invalid content_range, str was {}", range))
    }
}

/// Tries to resume an upload if an error happened
/// gets `id` which is the file id, `loc` which is the resumeable url and `f` which is the file
/// See: <https://developers.google.com/drive/v3/web/resumable-upload#resume-upload>
fn resume_upload(loc: &str, mut f: File, h: &HeaderMap) -> Result<()> {
    println!("Starting resume upload");
    let client = reqwest::blocking::Client::new();
    let mut header = h.clone();
    header.insert(CONTENT_RANGE, HeaderValue::from_static("*-*"));
    let res = client.put(loc).send()?;
    println!("Send put request");
    if (res.status() == reqwest::StatusCode::OK) | (res.status() == reqwest::StatusCode::CREATED) {
        Ok(())
    } else if res.status() == reqwest::StatusCode::NOT_FOUND {
        Err(anyhow!("Upload url not found, something is wrong"))
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
                    Err(anyhow!("We tried one resume and we could not finish"))
                }
            } else {
                Err(anyhow!("content range spec could not work"))
            }
        } else {
            Err(anyhow!("Could not find Content Range header"))
        }
    } else {
        Err(anyhow!("Unknown response returned"))
    }
}

/// Uploads a file to google drive to the directory given by `fileid`.
/// This uses the resubmeable upload feature by first uploading the metadata and then uploading the file via the resumeable url method.
/// Currently we do not support resuming a broken upload and just error out.
pub fn upload_file_or_local(
    tk: &Option<oauth2::basic::BasicTokenResponse>,
    f: File,
    paper: &Paper,
    fileid: &Option<String>,
    local_storage: &Option<String>,
) -> Result<()> {
    let filename = make_filename(paper);
    if let Some(dir) = local_storage {
        let mut file = f.try_clone().unwrap();
        let path = std::path::Path::new(dir).join(filename);
        let mut new_file = File::create(path)?;
        std::io::copy(&mut file, &mut new_file)?;
        Ok(())
    } else {
        let tk = tk.as_ref().expect("No local storage and no token");
        //getting the proper resumeable session URL
        info!("Uploading {}", &paper.title);
        let client = reqwest::blocking::Client::new();
        let mut header = HeaderMap::new();

        let authstring = "Bearer ".to_owned() + &tk.access_token().secret();
        header.insert(AUTHORIZATION, HeaderValue::from_str(&authstring).unwrap());

        let metadata = FileUploadJSON {
            name: filename,
            mime_type: "application/pdf".to_string(),
            parents: vec![String::from(fileid.as_ref().unwrap())],
        };

        let query = client
            .post(UPLOAD_URL)
            .headers(header.clone())
            .json(&metadata)
            .build();

        let res = client
            .execute(query.unwrap())
            .context("Error in getting resumeable url")?;

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
                Err(anyhow!("no location header found"))
            }
        } else {
            Err(anyhow!("Something went wrong with getting resumeable url"))
        }
    }
}
