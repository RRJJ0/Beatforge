use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

const UPDATE_STATE_SCHEMA_VERSION: u16 = 1;
const UPDATE_STATE_FILE: &str = "update-state.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateCheck {
    pub current_version: String,
    pub available_version: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateState {
    schema_version: u16,
    from_version: String,
    target_version: String,
    phase: String,
    attempted_at: String,
    error: Option<String>,
}

fn state_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join(UPDATE_STATE_FILE))
        .map_err(|error| format!("Could not resolve the update state path: {error}"))
}

fn write_state(path: &Path, state: &UpdateState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create the update state directory: {error}"))?;
    }
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| format!("Could not serialize update state: {error}"))?;
    fs::write(&temporary, bytes)
        .map_err(|error| format!("Could not write update state: {error}"))?;
    let backup = path.with_extension("json.backup");
    if path.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(path, &backup)
            .map_err(|error| format!("Could not preserve the previous update state: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(format!("Could not commit update state: {error}"));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

fn read_state(path: &Path) -> Result<Option<UpdateState>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| format!("Could not read update state: {error}"))?;
    let state: UpdateState = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Update state is invalid: {error}"))?;
    if state.schema_version != UPDATE_STATE_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported update state schema {}",
            state.schema_version
        ));
    }
    Ok(Some(state))
}

fn is_upgrade(current: &str, candidate: &str) -> Result<bool, String> {
    let current = Version::parse(current)
        .map_err(|error| format!("Current app version is invalid: {error}"))?;
    let candidate =
        Version::parse(candidate).map_err(|error| format!("Update version is invalid: {error}"))?;
    Ok(candidate > current)
}

#[cfg(test)]
fn verify_update_signature(
    payload: &[u8],
    wrapped_signature: &str,
    wrapped_public_key: &str,
    expected_version: &str,
) -> Result<(), String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use minisign_verify::{PublicKey, Signature};

    let public_key = STANDARD
        .decode(wrapped_public_key)
        .map_err(|error| format!("Could not decode updater public key: {error}"))?;
    let public_key =
        String::from_utf8(public_key).map_err(|_| "Updater public key is not UTF-8".to_string())?;
    let public_key = PublicKey::decode(&public_key)
        .map_err(|error| format!("Could not parse updater public key: {error}"))?;
    let signature = STANDARD
        .decode(wrapped_signature)
        .map_err(|error| format!("Could not decode updater signature: {error}"))?;
    let signature =
        String::from_utf8(signature).map_err(|_| "Updater signature is not UTF-8".to_string())?;
    let signature = Signature::decode(&signature)
        .map_err(|error| format!("Could not parse updater signature: {error}"))?;
    public_key
        .verify(payload, &signature, false)
        .map_err(|error| format!("Updater signature verification failed: {error}"))?;
    if !signature
        .trusted_comment()
        .split('\t')
        .any(|field| field == format!("version:{expected_version}"))
    {
        return Err("Updater signature is not bound to the expected version".into());
    }
    Ok(())
}

pub fn reconcile_on_start(app: &AppHandle) -> Result<(), String> {
    let path = state_path(app)?;
    let Some(mut state) = read_state(&path)? else {
        return Ok(());
    };
    let current = app.package_info().version.to_string();
    if reconcile_state(&current, &mut state) {
        write_state(&path, &state)?;
    }
    Ok(())
}

fn reconcile_state(current: &str, state: &mut UpdateState) -> bool {
    if current == state.target_version {
        state.phase = "completed".into();
        state.error = None;
        true
    } else if matches!(state.phase.as_str(), "downloading" | "applying") {
        state.phase = "failed".into();
        state.error = Some(
            "The updater returned control without activating the target version; the previous installation remains available. You can safely retry.".into(),
        );
        true
    } else {
        false
    }
}

#[tauri::command]
pub async fn check_for_app_update(app: AppHandle) -> Result<AppUpdateCheck, String> {
    let current = app.package_info().version.to_string();
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Could not initialize the updater: {error}"))?;
    match updater.check().await {
        Ok(Some(update)) => {
            if !is_upgrade(&current, &update.version)? {
                return Err(format!(
                    "Refusing update {} because it is not newer than installed version {current}",
                    update.version
                ));
            }
            Ok(AppUpdateCheck {
                current_version: current,
                available_version: Some(update.version),
                status: "available".into(),
                notes: update.body,
                published_at: update.date.map(|date| date.to_string()),
                message: None,
            })
        }
        Ok(None) => Ok(AppUpdateCheck {
            current_version: current,
            available_version: None,
            status: "up_to_date".into(),
            notes: None,
            published_at: None,
            message: None,
        }),
        Err(error) => Ok(AppUpdateCheck {
            current_version: current,
            available_version: None,
            status: "offline".into(),
            notes: None,
            published_at: None,
            message: Some(format!(
                "Could not reach the update service. Installed runtime and models remain usable offline: {error}"
            )),
        }),
    }
}

#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("Could not initialize the updater: {error}"))?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("Could not check for an update: {error}"))?
        .ok_or_else(|| "No newer Beatforge update is available".to_string())?;
    let current = app.package_info().version.to_string();
    if !is_upgrade(&current, &update.version)? {
        return Err(format!(
            "Refusing to install {} over {current}; Beatforge updates never downgrade user data",
            update.version
        ));
    }

    let path = state_path(&app)?;
    let mut state = UpdateState {
        schema_version: UPDATE_STATE_SCHEMA_VERSION,
        from_version: current,
        target_version: update.version.clone(),
        phase: "downloading".into(),
        attempted_at: chrono::Utc::now().to_rfc3339(),
        error: None,
    };
    write_state(&path, &state)?;
    state.phase = "applying".into();
    let result = update
        .download_and_install(|_, _| {}, || write_state(&path, &state).unwrap_or(()))
        .await;
    if let Err(error) = result {
        state.phase = "failed".into();
        state.error = Some(error.to_string());
        write_state(&path, &state)?;
        return Err(format!(
            "Update verification or installation failed; the current version remains installed: {error}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("beatforge-update-state-{nonce}.json"))
    }

    #[test]
    fn only_strictly_newer_semver_is_an_update() {
        assert!(is_upgrade("0.1.0", "0.1.1").unwrap());
        assert!(is_upgrade("0.1.0", "1.0.0").unwrap());
        assert!(!is_upgrade("1.0.0", "1.0.0").unwrap());
        assert!(!is_upgrade("1.0.0", "0.9.9").unwrap());
    }

    #[test]
    fn updater_signature_rejects_tampering_and_wrong_version() {
        const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDRDMjEyNTAyN0MzMzFGOTAKUldTUUh6TjhBaVVoVFBhUzNtdWdITHM5L0doRTV5ZzhZUndqYm5kbUZjUEd1M0JsMW1CaVR5N0MK";
        const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTUUh6TjhBaVVoVEMwdGp2SlNoS0MyajhJTU50enpkTGt2RTFiUTNkT1RndWkrd1k1RmNVdlpXQjZRZkJKa1ZxUXdOZDhoT2VBM1R3Ymx2c0YvdGRmNTArYUE5NmRmZ2dnPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkyMDUzNjgxCWZpbGU6dXBkYXRlci10ZXN0LXBheWxvYWQudHh0CXZlcnNpb246OS42LjAKRnRibHgwdy9jc1c1cTBWZlVNRVhUbFRqbmFsUFlQemNBdEgwaUQ3clREWXIzWHNYSUxaZEVVMDFicC8zUFAxSEU4YU1HSXpndTVUVlhMdEpaOUUvQWc9PQo=";
        const PAYLOAD: &[u8] = b"Beatforge updater verification fixture v1\n";

        verify_update_signature(PAYLOAD, SIGNATURE, PUBLIC_KEY, "9.6.0").unwrap();
        assert!(
            verify_update_signature(b"tampered", SIGNATURE, PUBLIC_KEY, "9.6.0")
                .unwrap_err()
                .contains("verification failed")
        );
        assert!(
            verify_update_signature(PAYLOAD, SIGNATURE, PUBLIC_KEY, "9.6.1")
                .unwrap_err()
                .contains("not bound")
        );
    }

    #[test]
    fn update_state_is_written_atomically_and_round_trips() {
        let path = temp_state_path();
        let state = UpdateState {
            schema_version: UPDATE_STATE_SCHEMA_VERSION,
            from_version: "0.1.0".into(),
            target_version: "0.2.0".into(),
            phase: "applying".into(),
            attempted_at: "2026-10-15T00:00:00Z".into(),
            error: None,
        };
        write_state(&path, &state).unwrap();
        let loaded = read_state(&path).unwrap().unwrap();
        assert_eq!(loaded.target_version, "0.2.0");
        assert_eq!(loaded.phase, "applying");
        assert!(!path.with_extension("json.tmp").exists());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn startup_reconciles_completed_and_interrupted_updates() {
        let mut interrupted = UpdateState {
            schema_version: UPDATE_STATE_SCHEMA_VERSION,
            from_version: "0.1.0".into(),
            target_version: "0.2.0".into(),
            phase: "downloading".into(),
            attempted_at: "2026-10-15T00:00:00Z".into(),
            error: None,
        };
        assert!(reconcile_state("0.1.0", &mut interrupted));
        assert_eq!(interrupted.phase, "failed");
        assert!(interrupted.error.unwrap().contains("safely retry"));

        let mut completed = UpdateState {
            schema_version: UPDATE_STATE_SCHEMA_VERSION,
            from_version: "0.1.0".into(),
            target_version: "0.2.0".into(),
            phase: "applying".into(),
            attempted_at: "2026-10-15T00:00:00Z".into(),
            error: Some("old error".into()),
        };
        assert!(reconcile_state("0.2.0", &mut completed));
        assert_eq!(completed.phase, "completed");
        assert!(completed.error.is_none());
    }
}
