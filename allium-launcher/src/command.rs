use std::process::Command;

pub enum AlliumCommand {
    Exec(Command),
}
