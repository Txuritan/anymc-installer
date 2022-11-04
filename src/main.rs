mod loaders;

mod gui;
mod utils;

use clap::Parser as _;

const ICON: &[u8] = include_bytes!("../icon.png");
const FABRIC_ICON: &[u8] = include_bytes!("../fabric.png");
const FORGE_ICON: &[u8] = include_bytes!("../forge.png");
const QUILT_ICON: &[u8] = include_bytes!("../quilt.png");

fn main() -> anyhow::Result<()> {
    if cfg!(target_os = "windows") {
        unsafe {
            enable_colors();
        }
    }

    tracing_subscriber::fmt().init();

    let args = Args::parse();

    if args.no_gui {
        println!("No gui mode")
    } else {
        gui::run(args)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn enable_colors() {
    use winapi::um::{
        consoleapi::SetConsoleMode, errhandlingapi::GetLastError, handleapi::INVALID_HANDLE_VALUE,
        processenv::GetStdHandle, winbase::STD_OUTPUT_HANDLE,
        wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    };

    let handle = GetStdHandle(STD_OUTPUT_HANDLE);
    if handle == INVALID_HANDLE_VALUE {
        let _err = GetLastError();
        return;
    }

    let _ = SetConsoleMode(handle, ENABLE_VIRTUAL_TERMINAL_PROCESSING);
}

#[derive(Default, clap::Parser)]
#[clap(about, version)]
pub struct Args {
    #[clap(long)]
    no_gui: bool,
}
