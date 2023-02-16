use crate::types::Paper;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::str;

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

/// Uploads a file to google drive to the directory given by `fileid`.
/// This uses the resubmeable upload feature by first uploading the metadata and then uploading the file via the resumeable url method.
/// Currently we do not support resuming a broken upload and just error out.
pub(crate) fn upload_file_or_local(
    //tk: &Option<oauth2::basic::BasicTokenResponse>,
    f: File,
    paper: &Paper,
    _fileid: &Option<String>,
    local_storage: &Option<String>,
) -> Result<()> {
    let filename = paper.filename();
    if let Some(dir) = local_storage {
        let mut file = f.try_clone().unwrap();
        let path = std::path::Path::new(dir).join(filename);
        let mut new_file = File::create(path)?;
        std::io::copy(&mut file, &mut new_file)?;
        Ok(())
    } else {
        Err(anyhow!("Not implemented"))
    }
}
