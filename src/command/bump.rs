use clap::{ArgGroup, Parser};
use semver::{BuildMetadata, Prerelease, Version};
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::{value, DocumentMut, Item};

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

#[derive(Debug)]
struct RustProject {
    root: PathBuf,
    manifest: PathBuf,
}

#[derive(Clone, Copy)]
enum VersionField {
    Package,
    WorkspacePackage,
}

pub fn execute(args: &Args) -> Result<()> {
    ensure_main_branch()?;
    let root = git_root()?;

    let current = latest_version()?;
    let next = bump_version(current.clone(), args.level())?;
    let current_tag = format!("v{current}");
    let next_tag = format!("v{next}");

    let needs_confirmation = args.major || (args.minor && !args.yes);
    if needs_confirmation && !confirm(&current_tag, &next_tag)? {
        println!("Canceled");
        return Ok(());
    }

    if let Some(project) = find_rust_project(root.clone())? {
        bump_rust_project(&project, &next)?;
    }

    create_tag(&root, &next_tag)?;

    println!("{next_tag}");
    Ok(())
}

fn create_tag(root: &Path, tag: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["tag", tag])
        .output()?;
    if !output.status.success() {
        return Err(command_error("git tag", &output.stderr).into());
    }
    Ok(())
}

fn git_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err(command_error("git rev-parse --show-toplevel", &output.stderr).into());
    }

    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()).canonicalize()?)
}

fn find_rust_project(root: PathBuf) -> Result<Option<RustProject>> {
    let mut directory = std::env::current_dir()?.canonicalize()?;
    if !directory.starts_with(&root) {
        return Err("current directory is outside the Git repository".into());
    }

    loop {
        let manifest = directory.join("Cargo.toml");
        if manifest.is_file() {
            return Ok(Some(RustProject { root, manifest }));
        }
        if directory == root || !directory.pop() {
            return Ok(None);
        }
    }
}

fn bump_rust_project(project: &RustProject, next: &Version) -> Result<()> {
    let lockfile = cargo_lockfile(project)?;
    ensure_project_files_clean(project, lockfile.as_deref())?;

    let content = fs::read_to_string(&project.manifest)?;
    let updated = update_manifest_version(&content, next)?;
    if updated == content {
        return Err(format!("Cargo.toml version is already {next}").into());
    }
    fs::write(&project.manifest, updated)?;
    if lockfile.is_some() {
        refresh_cargo_lock(project)?;
    }

    let manifest = project.manifest.strip_prefix(&project.root)?;
    let message = format!("🔧 chore(ver): 更新版本至 v{next}");
    let mut command = Command::new("git");
    command
        .current_dir(&project.root)
        .args(["commit", "--only", "-m", &message, "--"])
        .arg(manifest);
    if let Some(lockfile) = &lockfile {
        command.arg(lockfile.strip_prefix(&project.root)?);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(command_error("git commit", &output.stderr).into());
    }
    Ok(())
}

fn cargo_lockfile(project: &RustProject) -> Result<Option<PathBuf>> {
    let output = Command::new("cargo")
        .current_dir(&project.root)
        .args([
            "locate-project",
            "--workspace",
            "--message-format",
            "plain",
            "--manifest-path",
        ])
        .arg(&project.manifest)
        .output()?;
    if !output.status.success() {
        return Err(command_error("cargo locate-project", &output.stderr).into());
    }

    let workspace_manifest = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    let lockfile = workspace_manifest
        .parent()
        .ok_or("workspace Cargo.toml has no parent directory")?
        .join("Cargo.lock");
    if !lockfile.is_file() {
        return Ok(None);
    }
    let lockfile = lockfile.canonicalize()?;
    if !lockfile.starts_with(&project.root) {
        return Err("Cargo.lock is outside the Git repository".into());
    }
    Ok(Some(lockfile))
}

fn refresh_cargo_lock(project: &RustProject) -> Result<()> {
    let output = Command::new("cargo")
        .current_dir(&project.root)
        .args(["update", "--workspace", "--manifest-path"])
        .arg(&project.manifest)
        .output()?;
    if !output.status.success() {
        return Err(command_error("cargo update --workspace", &output.stderr).into());
    }
    Ok(())
}

fn ensure_project_files_clean(project: &RustProject, lockfile: Option<&Path>) -> Result<()> {
    let manifest = project.manifest.strip_prefix(&project.root)?;
    let mut command = Command::new("git");
    command
        .current_dir(&project.root)
        .args(["status", "--porcelain", "--"])
        .arg(manifest);
    if let Some(lockfile) = lockfile {
        command.arg(lockfile.strip_prefix(&project.root)?);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(command_error("git status", &output.stderr).into());
    }
    if !output.stdout.is_empty() {
        return Err("Cargo.toml or Cargo.lock has uncommitted changes".into());
    }
    Ok(())
}

fn update_manifest_version(content: &str, next: &Version) -> Result<String> {
    let mut document = content.parse::<DocumentMut>()?;
    let field = manifest_version_field(&document)
        .ok_or("Cargo.toml does not define a string version field")?;

    let current = match field {
        VersionField::Package => document["package"]["version"].as_str(),
        VersionField::WorkspacePackage => document["workspace"]["package"]["version"].as_str(),
    }
    .ok_or("Cargo.toml version must be a string")?;
    Version::parse(current)?;

    match field {
        VersionField::Package => document["package"]["version"] = value(next.to_string()),
        VersionField::WorkspacePackage => {
            document["workspace"]["package"]["version"] = value(next.to_string());
        }
    }
    Ok(document.to_string())
}

fn manifest_version_field(document: &DocumentMut) -> Option<VersionField> {
    if table_string(document.get("package"), "version").is_some() {
        return Some(VersionField::Package);
    }

    let workspace_package = document
        .get("workspace")
        .and_then(Item::as_table)
        .and_then(|workspace| workspace.get("package"));
    table_string(workspace_package, "version").map(|_| VersionField::WorkspacePackage)
}

fn table_string<'a>(item: Option<&'a Item>, key: &str) -> Option<&'a str> {
    item.and_then(Item::as_table)
        .and_then(|table| table.get(key))
        .and_then(Item::as_str)
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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn updates_package_manifest_version() {
        let manifest = "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n";
        let updated = update_manifest_version(manifest, &Version::new(1, 2, 4)).unwrap();
        assert!(updated.contains("version = \"1.2.4\""));
    }

    #[test]
    fn updates_workspace_manifest_version() {
        let manifest = "[workspace.package]\nversion = \"1.2.3\"\n";
        let updated = update_manifest_version(manifest, &Version::new(1, 3, 0)).unwrap();
        assert!(updated.contains("version = \"1.3.0\""));
    }

    #[test]
    fn commits_manifest_before_creating_tag() {
        let root = temp_repository();
        let manifest = root.join("Cargo.toml");
        fs::write(
            &manifest,
            "[package]\nname = \"demo\"\nversion = \"1.2.3\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        cargo(&root, &["generate-lockfile"]);
        git(&root, &["add", "Cargo.toml", "Cargo.lock", "src/main.rs"]);
        git(&root, &["commit", "-m", "initial"]);

        let project = RustProject {
            root: root.clone(),
            manifest,
        };
        bump_rust_project(&project, &Version::new(1, 2, 4)).unwrap();
        create_tag(&root, "v1.2.4").unwrap();

        let content = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"1.2.4\""));
        let lockfile = fs::read_to_string(root.join("Cargo.lock")).unwrap();
        assert!(lockfile.contains("name = \"demo\"\nversion = \"1.2.4\""));
        assert_eq!(
            git(&root, &["log", "-1", "--format=%s"]),
            "🔧 chore(ver): 更新版本至 v1.2.4"
        );
        assert_eq!(
            git(
                &root,
                &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]
            ),
            "Cargo.lock\nCargo.toml"
        );
        assert_eq!(
            git(&root, &["rev-parse", "HEAD"]),
            git(&root, &["rev-list", "-1", "v1.2.4"])
        );

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_repository() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "gh-mozhu-bump-test-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        git(&root, &["init", "-b", "main"]);
        git(&root, &["config", "user.name", "test"]);
        git(&root, &["config", "user.email", "test@example.com"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        root
    }

    fn git(root: &Path, args: &[&str]) -> String {
        run(root, "git", args)
    }

    fn cargo(root: &Path, args: &[&str]) -> String {
        run(root, "cargo", args)
    }

    fn run(root: &Path, program: &str, args: &[&str]) -> String {
        let output = Command::new(program)
            .current_dir(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}
