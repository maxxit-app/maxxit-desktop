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

#[cfg(test)]
mod tests {
    use super::*;

    // Public prehashed test vector from minisign-verify's upstream tests.
    const PUBLIC_KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";

    struct Fixture(std::path::PathBuf);

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn verifies_encoded_updater_envelope_and_rejects_tampering() {
        let root =
            Fixture(env::temp_dir().join(format!("maxxit-verifier-{}", uuid::Uuid::new_v4())));
        fs::create_dir(&root.0).unwrap();
        let archive = root.0.join("update.tar.gz");
        let signature = root.0.join("update.sig");
        let config = root.0.join("tauri.conf.json");
        fs::write(&archive, b"test").unwrap();
        fs::write(&signature, format!("{}\n", STANDARD.encode(SIGNATURE))).unwrap();
        fs::write(
            &config,
            serde_json::json!({"plugins":{"updater":{"pubkey":STANDARD.encode(PUBLIC_KEY)}}})
                .to_string(),
        )
        .unwrap();
        assert_eq!(verify(&archive, &signature, &config), Ok(()));

        fs::write(&archive, b"Test").unwrap();
        assert_eq!(
            verify(&archive, &signature, &config),
            Err("Updater signature verification failed".into())
        );
        fs::write(&archive, b"test").unwrap();
        fs::write(
            &signature,
            STANDARD.encode(SIGNATURE.replace("1556193335", "1556193336")),
        )
        .unwrap();
        assert!(verify(&archive, &signature, &config).is_err());
        fs::write(&signature, "invalid base64").unwrap();
        assert_eq!(
            verify(&archive, &signature, &config),
            Err("Invalid signature encoding".into())
        );
    }
}
