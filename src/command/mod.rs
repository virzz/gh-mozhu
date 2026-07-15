use clap::Subcommand;

pub mod bump;
pub mod commit;
pub use bump::{execute as Bump, Args as BumpArgs};
pub use commit::{execute as Commit, Args as CommitArgs, CommitType};

#[derive(Subcommand)]
pub enum Command {
    /// Bump version with a Git tag
    Bump(BumpArgs),

    /// Commit Format Message
    #[clap(alias = "c")]
    Commit(Box<CommitArgs>),
}
