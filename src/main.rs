mod loaders;

mod gui;
mod utils;

use clap::Parser as _;

const ICON: &[u8] = include_bytes!("../icon.png");
const FABRIC_ICON: &[u8] = include_bytes!("../fabric.png");
const FORGE_ICON: &[u8] = include_bytes!("../forge.png");
const QUILT_ICON: &[u8] = include_bytes!("../quilt.png");

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_ansi(false).init();

    let args = Args::parse();

    if args.no_gui {
        println!("No gui mode")
    } else {
        gui::run(args)?;
    }

    Ok(())
}

#[derive(Default, clap::Parser)]
#[clap(about, version)]
pub struct Args {
    #[clap(long)]
    no_gui: bool,
}
