use clap::{ArgGroup, Parser};
use semver::{BuildMetadata, Prerelease, Version};
use std::error::Error;
use std::io::{self, Write};
use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(group(ArgGroup::new("level").multiple(false)))]
pub struct Args {
    /// Bump the major version (always asks for confirmation)
    #[arg(long, group = "level")]
    major: bool,

    /// Bump the minor version
    #[arg(short = 'm', long, group = "level")]
    minor: bool,

    /// Bump the patch version (default)
    #[arg(short = 'p', long, group = "level")]
    patch: bool,

    /// Skip confirmation for a minor bump
    #[arg(short = 'y', long, requires = "minor")]
    yes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Major,
    Minor,
    Patch,
}

pub fn execute(args: &Args) -> Result<()> {
    ensure_main_branch()?;

    let current = latest_version()?;
    let next = bump_version(current.clone(), args.level())?;
    let current_tag = format!("v{current}");
    let next_tag = format!("v{next}");

    let needs_confirmation = args.major || (args.minor && !args.yes);
    if needs_confirmation && !confirm(&current_tag, &next_tag)? {
        println!("Canceled");
        return Ok(());
    }

    let output = Command::new("git").args(["tag", &next_tag]).output()?;
    if !output.status.success() {
        return Err(command_error("git tag", &output.stderr).into());
    }

    println!("{next_tag}");
    Ok(())
}

impl Args {
    fn level(&self) -> Level {
        if self.major {
            Level::Major
        } else if self.minor {
            Level::Minor
        } else {
            Level::Patch
        }
    }
}

fn ensure_main_branch() -> Result<()> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;
    if !output.status.success() {
        return Err(command_error("git branch --show-current", &output.stderr).into());
    }

    let branch = String::from_utf8(output.stdout)?.trim().to_owned();
    if branch != "main" {
        return Err(format!("bump only supports the main branch; current branch: {branch}").into());
    }
    Ok(())
}

fn latest_version() -> Result<Version> {
    let output = Command::new("git").args(["tag", "--list", "v*"]).output()?;
    if !output.status.success() {
        return Err(command_error("git tag --list", &output.stderr).into());
    }

    String::from_utf8(output.stdout)?
        .lines()
        .filter_map(parse_tag)
        .max()
        .ok_or_else(|| "no valid SemVer tag with prefix v found".into())
}

fn parse_tag(tag: &str) -> Option<Version> {
    tag.strip_prefix('v')
        .and_then(|value| Version::parse(value).ok())
}

fn bump_version(mut version: Version, level: Level) -> Result<Version> {
    match level {
        Level::Major => {
            version.major = version
                .major
                .checked_add(1)
                .ok_or("major version overflow")?;
            version.minor = 0;
            version.patch = 0;
        }
        Level::Minor => {
            version.minor = version
                .minor
                .checked_add(1)
                .ok_or("minor version overflow")?;
            version.patch = 0;
        }
        Level::Patch => {
            version.patch = version
                .patch
                .checked_add(1)
                .ok_or("patch version overflow")?;
        }
    }
    version.pre = Prerelease::EMPTY;
    version.build = BuildMetadata::EMPTY;
    Ok(version)
}

fn confirm(current: &str, next: &str) -> Result<bool> {
    print!("Bump {current} to {next}? [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn command_error(command: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr);
    let detail = detail.trim();
    if detail.is_empty() {
        format!("{command} failed")
    } else {
        format!("{command} failed: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_is_the_default_level() {
        let args = Args::parse_from(["bump"]);
        assert_eq!(args.level(), Level::Patch);
    }

    #[test]
    fn parses_only_prefixed_semver_tags() {
        assert_eq!(parse_tag("v1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(parse_tag("1.2.3"), None);
        assert_eq!(parse_tag("version-1.2.3"), None);
    }

    #[test]
    fn bumps_each_semver_level() {
        let version = Version::parse("1.2.3-alpha.1+build.5").unwrap();
        assert_eq!(
            bump_version(version.clone(), Level::Major).unwrap(),
            Version::new(2, 0, 0)
        );
        assert_eq!(
            bump_version(version.clone(), Level::Minor).unwrap(),
            Version::new(1, 3, 0)
        );
        assert_eq!(
            bump_version(version, Level::Patch).unwrap(),
            Version::new(1, 2, 4)
        );
    }
}
