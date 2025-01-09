use clap::{arg, command, Parser};

pub mod command;
pub use command::{Command, Commit, CommitArgs};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    debug: bool,

    #[command(subcommand)]
    command: Command,
}

fn main() {
    let cli = Args::parse();

    match &cli.command {
        Command::Commit(c) => Commit(c),
    }
}
