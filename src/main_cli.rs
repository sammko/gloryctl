use anyhow::Result;

use clap::Clap;
use gloryctl::GloriousDevice;

#[derive(Clap)]
pub struct Opts {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clap)]
enum Command {
    Dump(Dump),
}

#[derive(Clap)]
struct Dump {}

impl Dump {
    fn run(&self) -> Result<()> {
        let hid = hidapi::HidApi::new()?;
        let dev = GloriousDevice::open_first(&hid)?;

        dbg!(dev.read_fw_version()?);
        dbg!(dev.read_config()?);
        Ok(())
    }
}

fn main() -> Result<()> {
    //Dump {}.run()?;
    let opts = Opts::parse();

    match opts.cmd {
        Command::Dump(dump) => dump.run(),
    }
}
