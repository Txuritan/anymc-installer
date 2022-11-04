pub mod fabric;
pub mod forge;
pub mod minecraft;
pub mod quilt;

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(num_enum::IntoPrimitive, num_enum::FromPrimitive)]
#[repr(u8)]
pub enum Side {
    #[default]
    Client,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(num_enum::IntoPrimitive, num_enum::FromPrimitive)]
#[repr(u8)]
pub enum Loader {
    Fabric,
    Forge,
    #[default]
    Quilt,
}

#[derive(Debug, Clone)]
pub enum LoaderVersion {
    Fabric(fabric::Version),
    Forge(bool),
    Quilt(quilt::Version),
}

impl LoaderVersion {
    pub const fn name(&self) -> &'static str {
        match self {
            LoaderVersion::Fabric(_) => "fabric-loader",
            LoaderVersion::Forge(_) => "forge",
            LoaderVersion::Quilt(_) => "quilt-loader",
        }
    }

    pub const fn loader(&self) -> Loader {
        match self {
            LoaderVersion::Fabric(_) => Loader::Fabric,
            LoaderVersion::Forge(_) => Loader::Forge,
            LoaderVersion::Quilt(_) => Loader::Quilt,
        }
    }

    pub const fn icon(&self) -> &'static [u8] {
        match self.loader() {
            Loader::Fabric => crate::FABRIC_ICON,
            Loader::Forge => crate::FORGE_ICON,
            Loader::Quilt => crate::QUILT_ICON,
        }
    }
}

impl std::fmt::Display for LoaderVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderVersion::Fabric(version) => version.fmt(f),
            LoaderVersion::Forge(version) => version.fmt(f),
            LoaderVersion::Quilt(version) => version.fmt(f),
        }
    }
}

pub struct Install<V> {
    pub version: V,
    pub side: Side,
    pub dir: PathBuf,
    pub minecraft: minecraft::Version,
    pub generate: bool,
}

#[rustfmt::skip]
pub async fn install(
    loader: LoaderVersion,
    side: Side,
    dir: PathBuf,
    minecraft: minecraft::Version,
    generate: bool,
) -> anyhow::Result<()> {
    if !dir.exists() {
        anyhow::bail!("Installation directory doesn't exist: {}", dir.display());
    }

    match loader {
        LoaderVersion::Fabric(version) => {
            fabric::install(Install { side, dir, minecraft, version, generate }).await
        }
        LoaderVersion::Forge(version) => {
            forge::install(Install { side, dir, minecraft, version, generate }).await
        }
        LoaderVersion::Quilt(version) => {
            quilt::install(Install { side, dir, minecraft, version, generate }).await
        }
    }
}
