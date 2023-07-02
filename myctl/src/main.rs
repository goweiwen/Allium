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
                    Command::new("brightness").arg(
                        arg!([BRIGHTNESS] "Brightness to set")
                            .allow_negative_numbers(true)
                            .value_parser(value_parser!(i32)),
                    ),
                )
                .subcommand(
                    Command::new("lumination").arg(
                        arg!([LUMINATION] "Lumination to set")
                            .allow_negative_numbers(true)
                            .value_parser(value_parser!(i32)),
                    ),
                )
                .subcommand(
                    Command::new("hue").arg(
                        arg!([HUE] "Hue to set")
                            .allow_negative_numbers(true)
                            .value_parser(value_parser!(i32)),
                    ),
                )
                .subcommand(
                    Command::new("saturation").arg(
                        arg!([SATURATION] "Saturation to set")
                            .allow_negative_numbers(true)
                            .value_parser(value_parser!(i32)),
                    ),
                )
                .subcommand(
                    Command::new("contrast").arg(
                        arg!([CONTRAST] "Contrast to set")
                            .allow_negative_numbers(true)
                            .value_parser(value_parser!(i32)),
                    ),
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
                    ("brightness", sub_matches) => {
                        if let Some(brightness) = sub_matches.get_one::<i32>("BRIGHTNESS") {
                            display::set_brightness(*brightness)?;
                        } else {
                            println!("{}", display::get_brightness()?);
                        }
                    }
                    ("lumination", sub_matches) => {
                        if let Some(lumination) = sub_matches.get_one::<i32>("LUMINATION") {
                            display::set_lumination(*lumination)?;
                        } else {
                            println!("{}", display::get_lumination()?);
                        }
                    }
                    ("hue", sub_matches) => {
                        if let Some(hue) = sub_matches.get_one::<i32>("HUE") {
                            display::set_hue(*hue)?;
                        } else {
                            println!("{}", display::get_hue()?);
                        }
                    }
                    ("saturation", sub_matches) => {
                        if let Some(saturation) = sub_matches.get_one::<i32>("SATURATION") {
                            display::set_saturation(*saturation)?;
                        } else {
                            println!("{}", display::get_saturation()?);
                        }
                    }
                    ("contrast", sub_matches) => {
                        if let Some(contrast) = sub_matches.get_one::<i32>("CONTRAST") {
                            display::set_contrast(*contrast)?;
                        } else {
                            println!("{}", display::get_contrast()?);
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
