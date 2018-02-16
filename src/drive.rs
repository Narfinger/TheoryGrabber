use errors::*;
use oauth2;
use std::collections::HashMap;
use std::fs::File;
use reqwest;
use reqwest::header::{Headers, Authorization, Bearer};
use types::Paper;

static UPLOAD_URL: &'static str = "https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable";
static DIRECTORY_URL: &'static str = "https://www.googleapis.com/drive/v3/files";
static DIRECTORY_NAME: &'static str = "TheoryGrabber";

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
    parents: Vec<String>
} 

/// Returns the initial of the last name of an author.
fn get_last_name_initials(author: &str) -> char {
    let lastname = author.split_whitespace().nth(1).expect(
        "No lastname found?",
    ); //lastname
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
    let mut title = paper.title.clone();
    title.truncate(75);

    datestring.to_string() + "-" + &author_string(paper) + "-" + &title + ".pdf"
}

/// Creates directory in google drive. If called multiple times, will create multiple directories and saves the last directory id to the configuration file.
pub fn create_directory(tk: &oauth2::Token) -> Result<String> {
    let client = reqwest::Client::new();
    let mut header = Headers::new();

    header.set(Authorization(Bearer { token: tk.access_token.to_owned() }));
    let mut metadata = HashMap::new();
    metadata.insert("name", DIRECTORY_NAME);
    metadata.insert("mimeType", "application/vnd.google-apps.folder");

    let mut res = client
        .post(DIRECTORY_URL)
        .headers(header.clone())
        .json(&metadata)
        .send().chain_err(|| "Error in sending to create directory")?;
    
    let response: FileCreateResponse = res.json().chain_err(|| "Error in decoding Response")?;

    Ok(response.id)
}

/// Uploads a file to google drive to the directory given by `fileid`.
/// This uses the resubmeable upload feature by first uploading the metadata and then uploading the file via the resumeable url method.
/// Currently we do not support resuming a broken upload and just error out.
pub fn upload_file(tk: &oauth2::Token, f: File, paper: &Paper, fileid: String) -> Result<()> {
    //getting the proper resumeable session URL
    let client = reqwest::Client::new();
    let mut header = Headers::new();

    header.set(Authorization(Bearer { token: tk.access_token.to_owned() }));

    let filename = make_filename(paper);

    let metadata = FileUploadJSON {
        name: filename,
        mime_type: "application/pdf".to_string(),
        parents: vec![fileid],
    };
    
    let query = client
        .post(UPLOAD_URL)
        .headers(header.clone())
        .json(&metadata)
        .build();

    let res = client.execute(query.unwrap()).chain_err(
        || "Error in getting resumeable url",
    )?;

    if res.status().is_success() {
        if let Some(loc) = res.headers().get::<reqwest::header::Location>() {
            client
                .put(&loc.to_string())
                .headers(header)
                .body(f)
                .send()
                .chain_err(|| "Error in uploading file to resumeable url")?;
            Ok(())
        } else {
            Err("no location header found".into())
        }
    } else {
        Err("something went wrong with getting resumeable url".into())
    }
}
