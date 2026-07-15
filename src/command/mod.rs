use clap::Subcommand;

pub mod bump;
pub mod commit;
pub use bump::{Args as BumpArgs, execute as Bump};
pub use commit::{Args as CommitArgs, CommitType, execute as Commit};

#[derive(Subcommand)]
pub enum Command {
    /// Bump version with a Git tag
    Bump(BumpArgs),

    /// Commit Format Message
    #[clap(alias = "c")]
    Commit(Box<CommitArgs>),
}
