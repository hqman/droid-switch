use std::process::ExitCode;

use clap::Parser;

use dsw::cli::{Cli, Command};
use dsw::commands;
use dsw::paths::Paths;
use dsw::paths::AUTH_FILES;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let paths = Paths::from_env();

    let Some(command) = cli.command else {
        print_onboarding(&paths);
        return ExitCode::SUCCESS;
    };

    let res = match command {
        Command::Init(a) => commands::init::run(&paths, a),
        Command::Add(a) => commands::add::run(&paths, a),
        Command::Import(a) => commands::import::run(&paths, a),
        Command::Use(a) => commands::use_::run(&paths, a),
        Command::List(a) => commands::list::run(&paths, a),
        Command::Status(a) => commands::status::run(&paths, a),
        Command::Sync(a) => commands::sync::run(&paths, a),
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

fn print_onboarding(paths: &Paths) {
    println!("Droid Switch (dsw)");
    println!();
    if has_live_login(paths) {
        println!("A Droid login was found in {}.", paths.factory.display());
        println!();
        println!("Start by saving it as your main profile:");
        println!("  dsw import main");
    } else {
        println!("No Droid login was found in {}.", paths.factory.display());
        println!();
        println!("Log in first, then save the account:");
        println!("  droid");
        println!("  dsw import main");
        println!();
        println!("Or let dsw launch Droid for a new profile:");
        println!("  dsw add main");
    }
    println!();
    println!("Useful commands:");
    println!("  dsw list");
    println!("  dsw use <name>");
    println!("  dsw status");
}

fn has_live_login(paths: &Paths) -> bool {
    AUTH_FILES.iter().any(|f| paths.factory.join(f).is_file())
}
