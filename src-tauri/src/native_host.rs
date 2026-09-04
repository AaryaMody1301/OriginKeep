use crate::{model::DownloadCapture, storage};
use serde::Serialize;
use std::io::{self, Read, Write};

#[derive(Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
}

pub fn run() -> Result<(), String> {
    let database = storage::default_database_path()?;
    storage::initialize_database(&database)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    loop {
        let mut length_bytes = [0_u8; 4];
        match reader.read_exact(&mut length_bytes) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(error.to_string()),
        }

        let length = u32::from_le_bytes(length_bytes) as usize;
        if length == 0 || length > 1024 * 1024 {
            write_response(
                &mut writer,
                &ErrorResponse {
                    ok: false,
                    error: "Native message length is outside the 1 MiB OriginKeep limit".into(),
                },
            )?;
            continue;
        }

        let mut payload = vec![0_u8; length];
        reader
            .read_exact(&mut payload)
            .map_err(|error| error.to_string())?;

        match serde_json::from_slice::<DownloadCapture>(&payload) {
            Ok(capture) => match storage::ingest_capture(&database, &capture) {
                Ok(result) => write_response(&mut writer, &result)?,
                Err(error) => write_response(&mut writer, &ErrorResponse { ok: false, error })?,
            },
            Err(error) => write_response(
                &mut writer,
                &ErrorResponse {
                    ok: false,
                    error: format!("Invalid capture payload: {error}"),
                },
            )?,
        }
    }
}

fn write_response<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<(), String> {
    let payload = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let length = u32::try_from(payload.len()).map_err(|_| "Response too large".to_string())?;
    writer
        .write_all(&length.to_le_bytes())
        .and_then(|_| writer.write_all(&payload))
        .and_then(|_| writer.flush())
        .map_err(|error| error.to_string())
}
