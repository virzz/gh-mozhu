use clap::Parser;

pub mod command;
pub use command::{Bump, BumpArgs, Command, Commit, CommitArgs};

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
        Command::Bump(args) => {
            if let Err(error) = Bump(args) {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
        Command::Commit(c) => Commit(c),
    }
}
