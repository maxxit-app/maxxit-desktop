use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{env, fs, path::Path, process::ExitCode};

fn verify(archive: &Path, signature: &Path, config: &Path) -> Result<(), String> {
    let data = fs::read(archive).map_err(|_| "Could not read updater archive")?;
    let raw = fs::read_to_string(signature).map_err(|_| "Could not read updater signature")?;
    let configuration: serde_json::Value = serde_json::from_slice(
        &fs::read(config).map_err(|_| "Could not read Tauri configuration")?,
    )
    .map_err(|_| "Invalid Tauri configuration")?;
    let key = configuration
        .pointer("/plugins/updater/pubkey")
        .and_then(|v| v.as_str())
        .ok_or("Missing updater public key")?;
    let key = STANDARD
        .decode(key)
        .map_err(|_| "Invalid updater public key encoding")?;
    let key = String::from_utf8(key).map_err(|_| "Invalid updater public key text")?;
    let signature = STANDARD
        .decode(raw.trim())
        .map_err(|_| "Invalid signature encoding")?;
    let signature = String::from_utf8(signature).map_err(|_| "Invalid signature text")?;
    PublicKey::decode(&key)
        .map_err(|_| "Invalid updater public key")?
        .verify(
            &data,
            &Signature::decode(&signature).map_err(|_| "Invalid signature")?,
            true,
        )
        .map_err(|_| "Updater signature verification failed".into())
}

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if arguments.len() != 3 {
        eprintln!("Usage: verify_updater ARCHIVE SIGNATURE TAURI_CONFIG");
        return ExitCode::FAILURE;
    }
    match verify(
        Path::new(&arguments[0]),
        Path::new(&arguments[1]),
        Path::new(&arguments[2]),
    ) {
        Ok(()) => {
            println!("Updater signature verified.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
