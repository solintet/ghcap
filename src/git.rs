use crate::process;
use std::{
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
};

fn line(s: String) -> String {
    s.strip_suffix('\n').unwrap_or(&s).to_owned()
}

pub fn git(args: &[&str]) -> Result<String, String> {
    git_in(None, args)
}
pub fn git_in(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let o = process::capture("git", args, dir)?;
    if o.success {
        Ok(line(o.stdout))
    } else {
        Err(process::failure("git", &o))
    }
}
pub fn git_in_live(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let o = process::live("git", args, dir)?;
    if o.success {
        Ok(o.stdout)
    } else {
        Err(process::failure("git", &o))
    }
}
pub fn root(path: &Path) -> Result<PathBuf, String> {
    Ok(PathBuf::from(git_in(
        Some(path),
        &["rev-parse", "--show-toplevel"],
    )?))
}
pub fn validate_root(path: &Path) -> Result<(), String> {
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
        "GIT_NAMESPACE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_SHALLOW_FILE",
    ] {
        if std::env::var_os(key).is_some() {
            return Err(format!("{key} overrides Git's target. Use an ordinary repository environment; no settings were changed."));
        }
    }
    let expected = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let actual = fs::canonicalize(root(path)?).map_err(|e| e.to_string())?;
    if actual != expected {
        return Err(format!(
            "Selected folder is inside another repository: {}. Select its root explicitly.",
            actual.display()
        ));
    }
    Ok(())
}
pub fn current_repo() -> Option<String> {
    git(&["rev-parse", "--show-toplevel"]).ok()
}
pub fn remote_origin() -> Option<String> {
    git(&["remote", "get-url", "origin"]).ok()
}
pub fn is_repo(path: &Path) -> bool {
    root(path).is_ok()
}
pub fn has_origin(path: &Path) -> bool {
    origin(path).is_some()
}
pub fn origin(path: &Path) -> Option<String> {
    git_in(Some(path), &["remote", "get-url", "origin"]).ok()
}
pub fn init(path: &Path) -> Result<String, String> {
    if is_repo(path) {
        validate_root(path)?;
        return Ok("Already a repository.".into());
    }
    git_in(Some(path), &["init"])
}
pub fn set_origin(path: &Path, url: &str) -> Result<String, String> {
    validate_root(path)?;
    if has_origin(path) {
        git_in(Some(path), &["remote", "set-url", "origin", url])
    } else {
        git_in(Some(path), &["remote", "add", "origin", url])
    }
}
pub fn set_signing_key(path: &Path, key: &str) -> Result<String, String> {
    validate_root(path)?;
    git_in(Some(path), &["config", "user.signingkey", key])
}
pub fn user_name(path: &Path) -> Option<String> {
    git_in(Some(path), &["config", "user.name"])
        .ok()
        .filter(|s| !s.trim().is_empty())
}
pub fn user_email(path: &Path) -> Option<String> {
    git_in(Some(path), &["config", "user.email"])
        .ok()
        .filter(|s| !s.trim().is_empty())
}
pub fn identity_configured(path: &Path) -> bool {
    user_name(path).is_some() && user_email(path).is_some()
}
pub fn set_identity(path: &Path, name: &str, email: &str) -> Result<String, String> {
    validate_root(path)?;
    git_in(Some(path), &["config", "user.name", name])?;
    git_in(Some(path), &["config", "user.email", email])
}

pub fn github_identity(url: &str) -> Option<String> {
    let url = url.trim_end_matches('/').trim_end_matches(".git");
    let rest = [
        "https://github.com/",
        "http://github.com/",
        "git@github.com:",
        "ssh://git@github.com/",
    ]
    .iter()
    .find_map(|p| url.strip_prefix(p))?;
    let parts: Vec<_> = rest.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || p.starts_with('-')
                || !p
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    {
        return None;
    }
    Some(rest.to_ascii_lowercase())
}
pub fn same_remote(a: &str, b: &str) -> bool {
    match (github_identity(a), github_identity(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}
fn config(path: &Path, key: &str) -> Result<Option<String>, String> {
    let o = process::capture("git", &["config", "--get-all", key], Some(path))?;
    match o.code {
        0 if o.success => Ok(Some(line(o.stdout))),
        1 if matches!(o.termination, process::Termination::Exited(1)) => Ok(None),
        _ => Err(process::failure("git config", &o)),
    }
}
pub fn preflight_commit(path: &Path) -> Result<(), String> {
    validate_root(path)?;
    if git_in(Some(path), &["branch", "--show-current"])?.is_empty() {
        return Err("Detached HEAD: create/select a branch using Git before committing. Nothing was staged.".into());
    }
    let unmerged = git_in(Some(path), &["ls-files", "-u"])?;
    if !unmerged.is_empty() {
        return Err(
            "Unresolved merge conflicts: resolve them explicitly using Git. Nothing was staged."
                .into(),
        );
    }
    let gd = PathBuf::from(git_in(Some(path), &["rev-parse", "--absolute-git-dir"])?);
    for state in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ] {
        if gd.join(state).exists() {
            return Err(format!("Git operation in progress ({state}); finish it explicitly using Git first. Nothing was staged."));
        }
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushPlan {
    pub branch: String,
    pub remote: String,
    pub target: String,
    pub url: String,
    pub head: String,
}
pub fn push_plan(path: &Path) -> Result<PushPlan, String> {
    preflight_commit(path)?;
    let branch = git_in(Some(path), &["branch", "--show-current"])?;
    let head = git_in(Some(path), &["rev-parse", "--verify", "HEAD"])?;
    let remote = config(path, &format!("branch.{branch}.remote"))?
        .ok_or("No upstream remote. Set it explicitly using Git before Push.")?;
    if remote != "origin" {
        return Err(
            "Upstream is not origin. Review repository association using Git before Push.".into(),
        );
    }
    let target = config(path, &format!("branch.{branch}.merge"))?
        .ok_or("No upstream branch. Set it explicitly using Git before Push.")?;
    if !target.starts_with("refs/heads/") || target.contains('\n') {
        return Err("Invalid upstream ref.".into());
    }
    git_in(Some(path), &["check-ref-format", &target])?;
    if config(path, "remote.origin.mirror")?.is_some_and(|s| s != "false" && s != "no" && s != "0")
    {
        return Err(
            "Mirror Push is forbidden in the normal workflow; Git settings were preserved.".into(),
        );
    }
    if config(path, "push.default")?.as_deref() == Some("matching") {
        return Err(
            "push.default=matching is unsafe for this workflow. Review Git settings explicitly."
                .into(),
        );
    }
    if let Some(refspec) = config(path, "remote.origin.push")? {
        if refspec.lines().count() != 1
            || refspec.starts_with('+')
            || refspec.contains('*')
            || refspec.starts_with(':')
            || refspec != format!("refs/heads/{branch}:{target}")
        {
            return Err(
                "Configured Push refspec is ambiguous or unsafe; no refs were sent.".into(),
            );
        }
    }
    let urls = git_in(
        Some(path),
        &["remote", "get-url", "--push", "--all", "origin"],
    )?;
    if urls.lines().count() != 1 || urls.is_empty() {
        return Err("Multiple or missing Push URLs; no refs were sent.".into());
    }
    let fetch = git_in(Some(path), &["remote", "get-url", "origin"])?;
    if !same_remote(&fetch, &urls) {
        return Err(
            "Fetch and Push URLs identify different repositories. Review Git settings explicitly."
                .into(),
        );
    }
    Ok(PushPlan {
        branch,
        remote,
        target,
        url: urls,
        head,
    })
}
pub fn execute_push(path: &Path, plan: &PushPlan) -> Result<String, String> {
    if &push_plan(path)? != plan {
        return Err(
            "Push target or HEAD changed after confirmation. Review again; no refs were sent."
                .into(),
        );
    }
    git_in_live(
        Some(path),
        &[
            "push",
            "--porcelain",
            &plan.remote,
            &format!("{}:{}", plan.head, plan.target),
        ],
    )
}
/// Fingerprint the exact all-files commit candidate, including untracked bytes.
pub fn commit_fingerprint(path: &Path) -> Result<u64, String> {
    preflight_commit(path)?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let head = process::capture(
        "git",
        &["rev-parse", "--verify", "--quiet", "HEAD"],
        Some(path),
    )?;
    match head.termination {
        process::Termination::Exited(0) if head.success => head.stdout.hash(&mut h),
        process::Termination::Exited(1) => "unborn".hash(&mut h),
        _ => return Err(process::failure("git rev-parse", &head)),
    }
    git_in(Some(path), &["symbolic-ref", "HEAD"])?.hash(&mut h);
    let status = git_in(
        Some(path),
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ],
    )?;
    let mut names = std::collections::BTreeSet::new();
    for entry in status.split('\0').filter(|v| !v.is_empty()) {
        if entry.len() < 3 {
            return Err("Invalid Git status response.".into());
        }
        let name = &entry[3..];
        if name.chars().any(char::is_control) {
            return Err("Control characters in changed paths require explicit Git review.".into());
        }
        names.insert(name);
        if names.len() > 10000 {
            return Err(
                "Too many changed files to review safely; use Git for this operation.".into(),
            );
        }
    }
    for name in names {
        name.hash(&mut h);
        let file = path.join(name);
        match fs::symlink_metadata(&file) {
            Ok(meta) if meta.file_type().is_symlink() => {
                "symlink".hash(&mut h);
                fs::read_link(&file)
                    .map_err(|e| e.to_string())?
                    .hash(&mut h);
            }
            Ok(meta) if meta.is_file() => {
                "file".hash(&mut h);
                meta.len().hash(&mut h);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    (meta.permissions().mode() & 0o111).hash(&mut h);
                }
                let mut input = fs::File::open(&file).map_err(|e| e.to_string())?;
                let mut b = [0u8; 8192];
                loop {
                    let n = input.read(&mut b).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    h.write(&b[..n]);
                }
            }
            Ok(_) => return Err("Submodule/directory changes require explicit Git review.".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "deleted".hash(&mut h),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(h.finish())
}
pub fn status_for(path: &Path) -> String {
    match status_result(path) {
        Ok(s) => s,
        Err(e) => format!("Status unavailable:\n{}", process::safe_text(&e)),
    }
}
fn status_result(path: &Path) -> Result<String, String> {
    validate_root(path)?;
    let status = git_in(
        Some(path),
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
    )?;
    let mut headers = Vec::new();
    let mut count = 0;
    let mut skip = false;
    for row in status.split('\0').filter(|s| !s.is_empty()) {
        if skip {
            skip = false;
            continue;
        }
        if row.starts_with('#') {
            headers.push(row.to_owned());
        } else {
            count += 1;
            if row.starts_with("2 ") {
                skip = true;
            }
        }
    }
    Ok(format!("{}\nChanged files: {count}", headers.join("\n")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn repo() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        git_in(Some(d.path()), &["init", "-b", "master"]).unwrap();
        for (k, v) in [
            ("user.name", "Audit"),
            ("user.email", "audit@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.fsmonitor", "false"),
        ] {
            git_in(Some(d.path()), &["config", k, v]).unwrap();
        }
        git_in(
            Some(d.path()),
            &[
                "config",
                "core.hooksPath",
                d.path().join("no-hooks").to_str().unwrap(),
            ],
        )
        .unwrap();
        fs::write(d.path().join("file"), "base\n").unwrap();
        git_in(Some(d.path()), &["add", "."]).unwrap();
        git_in(Some(d.path()), &["commit", "-m", "initial"]).unwrap();
        d
    }
    pub(crate) fn remote(d: &tempfile::TempDir) -> tempfile::TempDir {
        let bare = tempfile::tempdir().unwrap();
        git_in(Some(bare.path()), &["init", "--bare"]).unwrap();
        git_in(
            Some(d.path()),
            &["remote", "add", "origin", bare.path().to_str().unwrap()],
        )
        .unwrap();
        git_in(Some(d.path()), &["push", "-u", "origin", "master"]).unwrap();
        bare
    }
    #[test]
    fn parent_root_not_accepted() {
        let d = repo();
        let sub = d.path().join("child");
        fs::create_dir(&sub).unwrap();
        assert!(validate_root(&sub).is_err());
        assert!(set_origin(&sub, "https://example.invalid/repo").is_err());
        assert!(!has_origin(d.path()));
    }
    #[test]
    fn trailing_space_root_preserved() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("repo ");
        fs::create_dir(&p).unwrap();
        git_in(Some(&p), &["init"]).unwrap();
        let found = root(&p).unwrap();
        assert_eq!(found.file_name().unwrap(), "repo ");
        assert_eq!(
            fs::canonicalize(found).unwrap(),
            fs::canonicalize(&p).unwrap()
        );
    }
    #[test]
    fn unknown_status_is_not_clean() {
        let d = tempfile::tempdir().unwrap();
        assert!(status_for(&d.path().join("absent")).contains("unavailable"));
    }
    #[test]
    fn mirror_does_not_delete_remote_ref() {
        let d = repo();
        let bare = remote(&d);
        let sha = git_in(Some(d.path()), &["rev-parse", "HEAD"]).unwrap();
        git_in(
            Some(bare.path()),
            &["update-ref", "refs/heads/remote-only", &sha],
        )
        .unwrap();
        git_in(Some(d.path()), &["config", "remote.origin.mirror", "true"]).unwrap();
        assert!(push_plan(d.path()).is_err());
        assert_eq!(
            git_in(Some(bare.path()), &["rev-parse", "remote-only"]).unwrap(),
            sha
        );
    }
    #[test]
    fn force_refspec_rejected() {
        let d = repo();
        let _bare = remote(&d);
        git_in(
            Some(d.path()),
            &[
                "config",
                "remote.origin.push",
                "+refs/heads/master:refs/heads/master",
            ],
        )
        .unwrap();
        assert!(push_plan(d.path()).is_err());
    }
    #[test]
    fn detached_is_rejected() {
        let d = repo();
        git_in(Some(d.path()), &["checkout", "--detach"]).unwrap();
        assert!(preflight_commit(d.path()).is_err());
    }
    #[test]
    fn unresolved_conflict_is_preserved() {
        let d = repo();
        git_in(Some(d.path()), &["checkout", "-b", "side"]).unwrap();
        fs::write(d.path().join("file"), "side\n").unwrap();
        git_in(Some(d.path()), &["commit", "-am", "side"]).unwrap();
        git_in(Some(d.path()), &["checkout", "master"]).unwrap();
        fs::write(d.path().join("file"), "master\n").unwrap();
        git_in(Some(d.path()), &["commit", "-am", "master"]).unwrap();
        assert!(git_in(Some(d.path()), &["merge", "side"]).is_err());
        let before = git_in(Some(d.path()), &["ls-files", "-u"]).unwrap();
        assert!(commit_fingerprint(d.path()).is_err());
        assert_eq!(before, git_in(Some(d.path()), &["ls-files", "-u"]).unwrap());
    }
    #[test]
    fn snapshot_detects_untracked_content_change() {
        let d = repo();
        let p = d.path().join("private");
        fs::write(&p, "first").unwrap();
        let before = commit_fingerprint(d.path()).unwrap();
        fs::write(&p, "other").unwrap();
        assert_ne!(before, commit_fingerprint(d.path()).unwrap());
    }
    #[test]
    fn push_plan_detects_target_change() {
        let d = repo();
        let _bare = remote(&d);
        let plan = push_plan(d.path()).unwrap();
        git_in(
            Some(d.path()),
            &["config", "branch.master.merge", "refs/heads/other"],
        )
        .unwrap();
        assert!(execute_push(d.path(), &plan).is_err());
    }
    #[test]
    fn validated_local_push_succeeds() {
        let d = repo();
        let bare = remote(&d);
        fs::write(d.path().join("new"), "new").unwrap();
        git_in(Some(d.path()), &["add", "."]).unwrap();
        git_in(Some(d.path()), &["commit", "-m", "next"]).unwrap();
        let plan = push_plan(d.path()).unwrap();
        execute_push(d.path(), &plan).unwrap();
        assert_eq!(
            git_in(Some(bare.path()), &["rev-parse", "master"]).unwrap(),
            plan.head
        );
    }
}
