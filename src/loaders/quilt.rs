use std::{
    borrow::Cow,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use futures::{stream, StreamExt, TryStreamExt};
use iced::{pick_list, Alignment, Checkbox, Column, Command, Element, Length, PickList, Row, Text};
use iced_native::command::Action;
use tokio::fs::File;
use zip::{write::FileOptions, CompressionMethod};

use crate::{
    loaders::{fabric, minecraft, Install, LoaderVersion, Side},
    utils,
};

pub static GAME: &str = "https://meta.quiltmc.org/v3/versions/game";
pub static MAVEN: &str = "https://maven.quiltmc.org/repository/release";
pub static META: &str = "https://meta.quiltmc.org/v3/versions/loader";

pub struct Commands;

impl Commands {
    #[tracing::instrument(skip_all, err)]
    pub async fn fetch_minecraft() -> anyhow::Result<Vec<minecraft::Version>> {
        Ok(reqwest::get(GAME).await?.json().await?)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn fetch_versions() -> anyhow::Result<Vec<Version>> {
        Ok(reqwest::get(META).await?.json().await?)
    }
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ClientProfile {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Arguments,
    pub libraries: Vec<Library>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerProfile {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub server_profile_type: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "launcherMainClass")]
    pub launcher_main_class: String,
    pub arguments: Arguments,
    pub libraries: Vec<Library>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arguments {
    pub game: Vec<Option<serde_json::Value>>,
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Library {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Deserialize)]
pub struct Version {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.version.fmt(f)
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn install(install: Install<Version>) -> anyhow::Result<()> {
    match install.side {
        Side::Client => install_client(install).await?,
        Side::Server => install_server(install).await?,
    }

    Ok(())
}

#[tracing::instrument(skip_all, err)]
async fn install_client(install: Install<Version>) -> anyhow::Result<()> {
    // Resolve profile directory
    let profile_name = format!("quilt-loader-{}-{}", install.version, install.minecraft);
    let mut profile_dir = install.dir.clone();
    profile_dir.push("versions");
    profile_dir.push(&profile_name);

    // Delete existing profile
    if profile_dir.exists() {
        tokio::fs::remove_dir_all(&profile_dir).await?;
    }

    // Create directory
    tokio::fs::create_dir_all(&profile_dir).await?;

    // NOTE: This is an empty jar file to make the vanilla launcher happy
    let mut jar_path = profile_dir.clone();
    jar_path.push(format!("{}.jar", &profile_name));
    File::create(jar_path).await?;

    // Create launch json
    let mut json_path = profile_dir.clone();
    json_path.push(format!("{}.json", &profile_name));
    let mut file = File::create(json_path).await?;

    // Download launch json
    let mut profile: ClientProfile = reqwest::get(format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
        &install.minecraft, &install.version
    ))
    .await?
    .json()
    .await?;

    // Hack-Fix:
    // Quilt-meta specifies both hashed and intermediary, but providing both to quilt-loader causes it to silently fail remapping.
    // This really shouldn't be fixed here in the installer, but we need a solution now.
    profile
        .libraries
        .retain(|lib| !lib.name.starts_with("org.quiltmc:hashed"));
    let response = serde_json::to_string_pretty(&profile)?;
    // End of hack-fix

    tokio::io::copy(&mut response.as_bytes(), &mut file).await?;

    if install.generate {
        utils::generate_profile(
            install.dir,
            install.minecraft,
            LoaderVersion::Quilt(install.version),
        )
        .await?;
    }

    Ok(())
}

#[tracing::instrument(skip_all, err)]
async fn install_server(install: Install<Version>) -> anyhow::Result<()> {
    // Download server json
    let mut profile: ServerProfile = reqwest::get(format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/server/json",
        &install.minecraft, &install.version
    ))
    .await?
    .json()
    .await?;

    // Hack-Fix:
    // Quilt-meta specifies both hashed and intermediary, but providing both to quilt-loader causes it to silently fail remapping.
    // This really shouldn't be fixed here in the installer, but we need a solution now.
    profile
        .libraries
        .retain(|lib| !lib.name.starts_with("org.quiltmc:hashed"));
    // End of hack-fix

    let libraries_dir = install.dir.to_path_buf().join("libraries");

    let client = reqwest::Client::new();

    let library_paths = tokio::spawn({
        let libraries = profile.libraries.clone();

        async move {
            let library_paths: anyhow::Result<Vec<PathBuf>> = stream::iter(libraries.into_iter())
                .map(|lib| {
                    let client = client.clone();
                    let libraries_dir = libraries_dir.clone();

                    async move { download_library(client, &libraries_dir, &lib).await }
                })
                .buffer_unordered(8)
                .try_collect()
                .await;

            library_paths
        }
    })
    .await??;

    let jar_path = install.dir.to_path_buf().join("quilt-server-launch.jar");
    create_launch_jar(&jar_path, &profile.launcher_main_class, &library_paths).await?;

    Ok(())
}

#[tracing::instrument(skip_all, err)]
async fn download_library(
    client: reqwest::Client,
    dir: &Path,
    lib: &Library,
) -> anyhow::Result<PathBuf> {
    fn split_artifact(artifact_notation: &str) -> Option<String> {
        let mut parts = artifact_notation.splitn(3, ':');

        let group = parts.next()?;
        let name = parts.next()?;
        let version = parts.next()?;

        let group = group.replace('.', "/");

        Some(format!(
            "{}/{}/{}/{}-{}.jar",
            group, name, version, name, version
        ))
    }

    let raw_path =
        split_artifact(&lib.name).context("Failed to build maven artifact from library name")?;
    let maven_url = match () {
        _ if raw_path.starts_with("org/quiltmc") => {
            format!("{}/{}", MAVEN, raw_path)
        }
        _ => {
            format!("{}/{}", fabric::MAVEN, raw_path)
        }
    };

    let path = dir.join(PathBuf::from(&raw_path));

    if path.exists() {
        tracing::info!(library = ?raw_path, "Library already downloaded, skipping...");
        return Ok(path);
    }

    let parent = path
        .parent()
        .expect("Install dir library has no parent folder");
    tokio::fs::create_dir_all(parent).await?;

    tracing::info!(library = ?raw_path, "Downloading library");

    let res = client.get(maven_url).send().await?;
    if !res.status().is_success() {
        anyhow::bail!(
            "Library download returned with status code: {}",
            res.status()
        );
    }

    let bytes = res.bytes().await?;
    tokio::fs::write(&path, &bytes[..]).await?;

    Ok(path)
}

#[tracing::instrument(skip_all, err)]
async fn create_launch_jar(jar: &Path, main: &str, libraries: &[PathBuf]) -> anyhow::Result<()> {
    tracing::info!("Creating server launch jar");

    let buf = Cursor::new(Vec::with_capacity(1024 * 2));
    let mut archive = zip::ZipWriter::new(buf);

    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    let parent = jar
        .parent()
        .expect("Server install directory has no parent");

    archive.start_file("META-INF/MANIFEST.MF", options)?;

    let mut manifest = Vec::new();
    writeln!(&mut manifest, "Manifest-Version: 1.0")?;
    writeln!(&mut manifest, "Main-Class: {}", main)?;

    let relative_paths = libraries
        .iter()
        .map(|path| {
            path.strip_prefix(parent)
                .context("Failed to make library path relative to install directory")
                .map(|path| path.display().to_string().replace('\\', "/"))
        })
        .collect::<anyhow::Result<Vec<String>>>()?;

    let class_path = format!("Class-Path: {}", relative_paths.join(" "));

    let (head, tail) = class_path.split_at(72);
    writeln!(&mut manifest, "{}", &head)?;

    for chunk in tail.as_bytes().chunks(71) {
        writeln!(&mut manifest, " {}", String::from_utf8_lossy(chunk))?;
    }

    archive.write_all(&manifest)?;

    let bytes = archive.finish()?;

    tokio::fs::write(jar, bytes.into_inner()).await?;

    Ok(())
}

#[derive(Debug)]
pub enum Message {
    Error(anyhow::Error),

    Minecraft(minecraft::Message),

    SetMinecraft(anyhow::Result<Vec<minecraft::Version>>),
    SetVersions(anyhow::Result<Vec<Version>>),
}

#[allow(clippy::from_over_into)]
impl Into<Command<Message>> for Message {
    fn into(self) -> Command<Message> {
        Command::single(Action::Future(Box::pin(async { self })))
    }
}

#[derive(Debug, Clone)]
pub enum Interaction {
    Minecraft(minecraft::Interaction),

    SelectVersion(Version),
    ShowBetas(bool),
}

#[derive(Debug, Default)]
pub struct State {
    pub minecraft: minecraft::State,

    pub pick_list: pick_list::State<Version>,
    pub versions: Vec<Version>,
    pub selected_version: Option<Version>,
    pub show_betas: bool,
}

impl State {
    pub fn selected_version(&self) -> Option<Version> {
        self.selected_version.as_ref().cloned()
    }

    pub fn selected_minecraft(&self) -> Option<minecraft::Version> {
        self.minecraft.selected_version.as_ref().cloned()
    }

    pub fn update_interaction(&mut self, interaction: Interaction) -> Command<Message> {
        match interaction {
            Interaction::Minecraft(interaction) => {
                return self
                    .minecraft
                    .update_interaction(interaction)
                    .map(Message::Minecraft)
            }

            Interaction::SelectVersion(version) => self.selected_version = Some(version),
            Interaction::ShowBetas(show) => self.show_betas = show,
        }

        Command::none()
    }

    pub fn update_message(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Error(err) => eprintln!("{:#?}", err),

            Message::Minecraft(message) => {
                return self
                    .minecraft
                    .update_message(message)
                    .map(Message::Minecraft)
            }

            Message::SetMinecraft(result) => {
                match result {
                    Ok(versions) => self.minecraft.versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }

                if self.minecraft.selected_version.is_none() {
                    self.minecraft.selected_version =
                        self.minecraft.versions.iter().find(|v| v.stable).cloned();
                }
            }
            Message::SetVersions(result) => {
                match result {
                    Ok(versions) => self.versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }

                if self.selected_version.is_none() {
                    self.selected_version = self
                        .versions
                        .iter()
                        .find(|v| !v.version.contains("beta"))
                        .cloned();
                }
            }
        }

        Command::none()
    }

    pub fn view(&mut self) -> Element<'_, Interaction> {
        Column::new()
            .push(self.minecraft.view().map(Interaction::Minecraft))
            .push(
                Row::new()
                    .push(Text::new("Loader version:").width(Length::Units(140)))
                    .push(
                        PickList::new(
                            &mut self.pick_list,
                            Cow::from_iter(
                                self.versions
                                    .iter()
                                    .filter(|v| self.show_betas || !v.version.contains("beta"))
                                    .cloned(),
                            ),
                            self.selected_version.clone(),
                            Interaction::SelectVersion,
                        )
                        .width(Length::Fill),
                    )
                    .push(Checkbox::new(
                        self.show_betas,
                        "Show betas",
                        Interaction::ShowBetas,
                    ))
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .spacing(5)
                    .padding(5),
            )
            .into()
    }
}
