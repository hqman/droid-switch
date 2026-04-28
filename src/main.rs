use std::process::ExitCode;

use clap::Parser;

use dsw::cli::{Cli, Command};
use dsw::commands;
use dsw::paths::Paths;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let paths = Paths::from_env();

    let res = match cli.command {
        Command::Init(a) => commands::init::run(&paths, a),
        Command::Add(a) => commands::add::run(&paths, a),
        Command::Import(a) => commands::import::run(&paths, a),
        Command::Use(a) => commands::use_::run(&paths, a),
        Command::List(a) => commands::list::run(&paths, a),
        Command::Status(a) => commands::status::run(&paths, a),
        Command::Remove(a) => commands::remove::run(&paths, a),
        Command::Rename(a) => commands::rename::run(&paths, a),
        Command::Doctor(a) => commands::doctor::run(&paths, a),
        Command::Backup(a) => commands::backup::run(&paths, a),
    };

    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
