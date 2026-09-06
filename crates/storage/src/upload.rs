use crate::filesystem::{AppFs, FsError};
use thiserror::Error;

#[derive(Debug)]
pub struct UploadResult {
    pub bytes_written: u64,
    pub csrf_token: String,
    pub original_filename: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("invalid multipart content type")]
    InvalidContentType,
    #[error("invalid multipart form")]
    InvalidMultipart,
    #[error("required multipart field is missing or duplicated")]
    FieldCardinality,
    #[error("multipart filename is invalid")]
    InvalidFilename,
    #[error(transparent)]
    Fs(#[from] FsError),
}

pub fn multipart_boundary(content_type: &str) -> Result<String, UploadError> {
    if !content_type
        .to_ascii_lowercase()
        .starts_with("multipart/form-data;")
    {
        return Err(UploadError::InvalidContentType);
    }
    multer::parse_boundary(content_type).map_err(|_| UploadError::InvalidContentType)
}

pub async fn store_single_multipart_file<R>(
    reader: R,
    boundary: &str,
    appfs: &AppFs,
    destination: &str,
    max_body_bytes: u64,
    expected_csrf: &str,
) -> Result<UploadResult, UploadError>
where
    R: tokio::io::AsyncRead + Unpin + Send,
{
    use multer::{Constraints, Multipart, SizeLimit};

    if !appfs.allows_create() {
        return Err(FsError::Denied.into());
    }
    appfs.validate_upload_destination(destination)?;
    let limits = SizeLimit::new()
        .whole_stream(max_body_bytes)
        .per_field(appfs.limits().max_file_bytes)
        .for_field("_csrf", 4096);
    let constraints = Constraints::new()
        .allowed_fields(vec!["_csrf", "file"])
        .size_limit(limits);
    let mut multipart =
        Multipart::with_reader_with_constraints(reader, boundary.to_owned(), constraints);
    let mut csrf: Option<String> = None;
    let mut csrf_verified = false;
    let mut upload: Option<(u64, Option<String>, Option<String>)> = None;
    let mut staged_path: Option<String> = None;

    let parsed: Result<(), UploadError> = async {
        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|_| UploadError::InvalidMultipart)?
        {
            let name = field
                .name()
                .ok_or(UploadError::InvalidMultipart)?
                .to_string();
            match name.as_str() {
                "_csrf" => {
                    if csrf.is_some() || upload.is_some() {
                        return Err(UploadError::FieldCardinality);
                    }
                    let text = field
                        .text()
                        .await
                        .map_err(|_| UploadError::InvalidMultipart)?;
                    if text.is_empty()
                        || text.len() > 4096
                        || text.bytes().any(|b| b < 0x21 || b > 0x7e)
                        || text != expected_csrf
                    {
                        return Err(UploadError::InvalidMultipart);
                    }
                    csrf_verified = true;
                    csrf = Some(text);
                }
                "file" => {
                    if !csrf_verified || upload.is_some() {
                        return Err(UploadError::FieldCardinality);
                    }
                    let filename = field
                        .file_name()
                        .map(validate_upload_filename)
                        .transpose()?;
                    let content_type = field.content_type().map(|v| v.to_string());
                    let (staging, mut out) = appfs.create_staged(destination).await?;
                    staged_path = Some(staging);
                    while let Some(chunk) = field
                        .chunk()
                        .await
                        .map_err(|_| UploadError::InvalidMultipart)?
                    {
                        out.write_chunk(&chunk).await?;
                    }
                    let bytes = out.finish().await?;
                    upload = Some((bytes, filename, content_type));
                }
                _ => return Err(UploadError::InvalidMultipart),
            }
        }
        Ok(())
    }
    .await;

    if let Err(err) = parsed {
        if let Some(staging) = staged_path.as_deref() {
            appfs.cleanup_staged(staging);
        }
        return Err(err);
    }
    let csrf_token = csrf.ok_or(UploadError::FieldCardinality)?;
    let (bytes_written, original_filename, content_type) =
        upload.ok_or(UploadError::FieldCardinality)?;
    let staging = staged_path
        .as_deref()
        .ok_or(UploadError::FieldCardinality)?;
    if let Err(err) = appfs.commit_staged(staging, destination) {
        appfs.cleanup_staged(staging);
        return Err(err.into());
    }
    Ok(UploadResult {
        bytes_written,
        csrf_token,
        original_filename,
        content_type,
    })
}

fn validate_upload_filename(raw: &str) -> Result<String, UploadError> {
    if raw.is_empty()
        || raw.len() > 255
        || raw.contains('/')
        || raw.contains('\\')
        || raw.contains('\0')
        || raw == "."
        || raw == ".."
    {
        return Err(UploadError::InvalidFilename);
    }
    if raw.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(UploadError::InvalidFilename);
    }
    Ok(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::store_single_multipart_file;
    use crate::{AppFs, FsLimits, FsMode};

    #[tokio::test]
    async fn streams_multipart_into_confined_file() {
        let root = std::env::temp_dir().join(format!("rwlang-appfs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("uploads")).unwrap();
        let fs = AppFs::open_root(
            &root,
            FsMode::parse("rwc").unwrap(),
            FsLimits {
                max_file_bytes: 1024,
                ..FsLimits::default()
            },
        )
        .unwrap();
        let body = b"--X\r\nContent-Disposition: form-data; name=\"_csrf\"\r\n\r\ntoken123\r\n--X\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\nhello\r\n--X--\r\n";
        let result = store_single_multipart_file(
            &body[..],
            "X",
            &fs,
            "uploads/u.bin",
            body.len() as u64 + 16,
            "token123",
        )
        .await
        .unwrap();
        assert_eq!(result.bytes_written, 5);
        assert_eq!(result.csrf_token, "token123");
        assert_eq!(fs.read("uploads/u.bin").await.unwrap(), b"hello");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
