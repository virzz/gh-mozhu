use clap::Subcommand;

pub mod commit;
pub use commit::{execute as Commit, Args as CommitArgs, CommitType};

#[derive(Subcommand)]
pub enum Command {
    /// Commit Format Message
    #[clap(alias = "c")]
    Commit(CommitArgs),
}
