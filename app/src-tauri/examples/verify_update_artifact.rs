use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use std::{env, fs, process};

fn verify() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: verify_update_artifact <tauri.conf.json> <installer> <signature> <version>"
                .into(),
        );
    }
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(&args[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let wrapped_key = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .ok_or("Missing updater public key")?;
    let key = STANDARD.decode(wrapped_key).map_err(|e| e.to_string())?;
    let key = PublicKey::decode(&String::from_utf8(key).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let wrapped_signature = fs::read_to_string(&args[3]).map_err(|e| e.to_string())?;
    let signature = STANDARD
        .decode(wrapped_signature.trim())
        .map_err(|e| e.to_string())?;
    let signature = Signature::decode(&String::from_utf8(signature).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let installer = fs::read(&args[2]).map_err(|e| e.to_string())?;
    key.verify(&installer, &signature, false)
        .map_err(|e| e.to_string())?;
    if !signature
        .trusted_comment()
        .split('\t')
        .any(|field| field == format!("version:{}", args[4]))
    {
        return Err("Updater signature is not bound to the artifact version".into());
    }
    println!("Updater artifact signature and version verified");
    Ok(())
}

fn main() {
    if let Err(error) = verify() {
        eprintln!("{error}");
        process::exit(1);
    }
}
