use anyhow::Result;
use clap::{arg, value_parser, Command};
use simple_logger::SimpleLogger;

mod volume;

fn cli() -> Command {
    Command::new(env!("CARGO_CRATE_NAME"))
        .about("Manages the Miyoo Mini hardware")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("volume")
                .arg(arg!([VOLUME] "Volume to set").value_parser(value_parser!(i32))),
        )
}

fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("volume", sub_matches)) => {
            if let Some(vol) = sub_matches.get_one::<i32>("VOLUME") {
                volume::set(*vol)?;
            } else {
                println!("{}", volume::get()?);
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
