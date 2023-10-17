use anyhow::Result;
use clap::{arg, value_parser, Command};
use simple_logger::SimpleLogger;

mod display;
mod volume;

fn cli() -> Command {
    Command::new(env!("CARGO_CRATE_NAME"))
        .about("Manages the Miyoo Mini hardware")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("volume").arg(
                arg!([VOLUME] "Volume to set")
                    .allow_negative_numbers(true)
                    .value_parser(value_parser!(i32)),
            ),
        )
        .subcommand(
            Command::new("display")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("blank")
                        .arg(arg!([TOGGLE] "blank or unblank").value_parser(value_parser!(bool))),
                ),
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
        Some(("display", sub_matches)) => {
            if let Some(sub_matches) = sub_matches.subcommand() {
                match sub_matches {
                    ("blank", sub_matches) => {
                        if let Some(true) = sub_matches.get_one::<bool>("BLANK") {
                            display::blank()?;
                        } else {
                            display::unblank()?;
                        }
                    }
                    _ => unreachable!(),
                }
            } else {
                unreachable!()
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
