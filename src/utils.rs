use std::{collections::HashMap, path::PathBuf};

use chrono::Utc;

use crate::loaders::{minecraft, LoaderVersion};

pub fn get_minecraft_directory() -> anyhow::Result<PathBuf> {
    let mut dir = PathBuf::from(std::env::var("APPDATA")?);

    if cfg!(target_os = "windows") {
        dir.push(".minecraft");
    } else if cfg!(target_os = "macos") {
        dir.push("Library");
        dir.push("Application Support");
        dir.push("minecraft");
    } else if cfg!(target_os = "linux") {
        dir.push(".minecraft");
    }

    Ok(dir)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LaunchProfiles {
    profiles: HashMap<String, serde_json::Value>,
    settings: serde_json::Value,
    version: u32,
}

pub async fn generate_profile(
    dir: PathBuf,
    minecraft: minecraft::Version,
    version: LoaderVersion,
) -> anyhow::Result<()> {
    let profile_name = format!("{}-{}-{}", version.name(), version, minecraft);

    let mut profiles_json = dir.clone();
    profiles_json.push("launcher_profiles.json");

    let read_file = tokio::fs::read_to_string(&profiles_json).await?;
    let mut profiles: LaunchProfiles = serde_json::from_str(&read_file)?;

    let new_profile = serde_json::json!({
        "name": format!("{}-{}", version.name(), &minecraft),
        "type": "custom",
        "created": format!("{:?}", Utc::now()),
        "lastVersionId": profile_name.clone(),
        "icon": format!("data:image/png;base64,{}", base64::encode(version.icon())),
    });

    profiles.profiles.insert(profile_name, new_profile);

    let new_profiles = serde_json::to_string_pretty(&profiles)?;
    tokio::fs::write(&profiles_json, &new_profiles).await?;

    Ok(())
}
