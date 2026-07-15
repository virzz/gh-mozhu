# gh-mozhu

## gh-commit

format commit messages for github

## bump

Create the next SemVer tag on `main`. Tags use the `v` prefix.
For Rust projects, `bump` updates the nearest `Cargo.toml` version and the
existing workspace `Cargo.lock`, commits both files, then creates the tag.

```shell
gh mozhu bump              # patch: v1.2.3 -> v1.2.4
gh mozhu bump --minor      # minor, with confirmation
gh mozhu bump -m --yes     # minor, without confirmation
gh mozhu bump --major      # major, always with confirmation
```
