use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "dsw",
    version,
    about = "Switch between multiple Factory droid accounts",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// First-run setup: create storage and import the current login as a profile
    Init(InitArgs),

    /// Launch `droid` to log in, then snapshot the result as a named profile
    Add(AddArgs),

    /// Snapshot the currently-live droid login as a named profile (no relogin)
    Import(ImportArgs),

    /// Activate a saved profile (auto-snapshots the previous active one)
    #[command(name = "use")]
    Use(UseArgs),

    /// List profiles, marking the active one
    #[command(alias = "ls")]
    List(ListArgs),

    /// Show the active profile and its email/expiry
    #[command(alias = "current", alias = "whoami")]
    Status(StatusArgs),

    /// Delete a saved profile
    #[command(alias = "rm")]
    Remove(RemoveArgs),

    /// Rename a saved profile
    Rename(RenameArgs),

    /// Diagnose the install (paths, permissions, token expiry)
    Doctor(DoctorArgs),

    /// Manage automatic backups created on every switch
    Backup(BackupArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Name to save the currently-live login as (skip if not logged in)
    #[arg(long)]
    pub import_as: Option<String>,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Profile name to create
    pub name: String,
    /// Skip launching `droid`; just snapshot whatever is currently active
    #[arg(long)]
    pub no_login: bool,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Profile name to create from the currently-live login
    pub name: String,
    /// Overwrite an existing profile with the same name
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct UseArgs {
    /// Profile name to activate
    pub name: String,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Profile name to delete
    pub name: String,
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    /// Existing profile name
    pub old: String,
    /// New profile name
    pub new: String,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct BackupArgs {
    #[command(subcommand)]
    pub command: BackupCommand,
}

#[derive(Subcommand, Debug)]
pub enum BackupCommand {
    /// List backups
    #[command(alias = "ls")]
    List,
    /// Restore a backup by id (timestamp directory name)
    Restore {
        /// Backup id (timestamp directory under ~/.dsw/backups/)
        id: String,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Prune backups older than N most recent (default: keep 10)
    Prune {
        /// How many recent backups to keep
        #[arg(long, default_value_t = 10)]
        keep: usize,
    },
}
