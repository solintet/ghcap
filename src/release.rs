use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
#[derive(Debug)]
pub struct ReleaseBundle {
    pub notes_file: PathBuf,
    pub assets: Vec<PathBuf>,
    _snapshot: tempfile::TempDir,
}
const MAX_FILE: u64 = 512 * 1024 * 1024;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024;
#[cfg(unix)]
fn open_beneath(root: &File, relative: &Path) -> Result<File, String> {
    use nix::{
        fcntl::{openat, OFlag},
        sys::stat::Mode,
    };
    use std::path::Component;
    let mut dir = root.try_clone().map_err(|e| e.to_string())?;
    let components = relative.components().collect::<Vec<_>>();
    for (i, part) in components.iter().enumerate() {
        let Component::Normal(name) = part else {
            return Err("Unsafe relative release path".into());
        };
        let mut flags = OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK;
        if i + 1 < components.len() {
            flags |= OFlag::O_DIRECTORY;
        }
        dir = File::from(
            openat(&dir, *name, flags, Mode::empty())
                .map_err(|e| format!("Cannot safely open release input: {e}"))?,
        );
    }
    Ok(dir)
}
#[cfg(not(unix))]
fn open_beneath(_root: &File, _relative: &Path) -> Result<File, String> {
    Err("Folder upload requires a verified no-follow file-opening implementation on this platform. Use the GitHub CLI directly; no files were uploaded.".into())
}
fn root_file(path: &Path) -> Result<File, String> {
    let mut o = fs::OpenOptions::new();
    o.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW);
    }
    o.open(path).map_err(|e| e.to_string())
}
pub fn validate_tag(tag: &str) -> Result<(), String> {
    if tag.is_empty()
        || tag.starts_with('-')
        || tag.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("Invalid release tag; leading options and whitespace are forbidden.".into());
    }
    crate::git::git(&["check-ref-format", &format!("refs/tags/{tag}")]).map(|_| ())
}
pub fn collect_bundle(directory: &Path) -> Result<ReleaseBundle, String> {
    if cfg!(not(unix)) {
        return Err(
            "Folder upload is disabled until safe no-follow support is verified on this platform."
                .into(),
        );
    }
    if fs::symlink_metadata(directory)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Select the real staging directory, not a symlink.".into());
    }
    let directory = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    let root = root_file(&directory)?;
    let snapshot = tempfile::Builder::new()
        .prefix("ghcap-release-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let mut notes = None;
    let mut assets = Vec::new();
    let mut names = HashSet::new();
    let mut total = 0;
    for entry in WalkDir::new(&directory)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0 || !matches!(e.file_name().to_str(), Some(".git" | ".svn" | "__MACOSX"))
        })
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.depth() == 0 || entry.file_type().is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&directory)
            .map_err(|e| e.to_string())?;
        let name = relative
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Non-UTF-8 release filename is unsupported")?;
        let is_notes = entry.depth() == 1
            && matches!(
                name,
                "release-notes.md" | "RELEASE_NOTES.md" | "Release-Notes.md"
            );
        if entry.file_type().is_symlink() {
            if is_notes {
                return Err("Release notes must be a regular file, never a symlink.".into());
            }
            continue;
        }
        if !entry.file_type().is_file() {
            return Err("Release inputs must be regular files.".into());
        }
        if name.contains('#')
            || relative.to_str().is_none()
            || relative.to_string_lossy().chars().any(char::is_control)
        {
            return Err("Release filenames must be UTF-8 without control characters or '#'. Rename the source explicitly; nothing was uploaded.".into());
        }
        if is_notes && notes.is_some() {
            return Err("Multiple release note files found; keep exactly one.".into());
        }
        if !is_notes && !names.insert(name.to_ascii_lowercase()) {
            return Err(format!("Duplicate asset filename '{name}'"));
        }
        if assets.len() >= 10000 {
            return Err("More than 10000 release assets; use a smaller staging set.".into());
        }
        let mut input = open_beneath(&root, relative)?;
        let before = input.metadata().map_err(|e| e.to_string())?;
        let limit = if is_notes { 1024 * 1024 } else { MAX_FILE };
        if !before.is_file() || before.len() > limit {
            return Err(
                "Release input is not a regular file or exceeds the per-file limit.".into(),
            );
        }
        total += before.len();
        if total > MAX_TOTAL {
            return Err("Staging set exceeds 1 GiB snapshot limit.".into());
        }
        let dest = snapshot.path().join(relative);
        if let Some(p) = dest.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            output
                .set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        }
        let copied = std::io::copy(
            &mut std::io::Read::by_ref(&mut input).take(limit + 1),
            &mut output,
        )
        .map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        let after = input.metadata().map_err(|e| e.to_string())?;
        if copied != before.len()
            || before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err("Release input changed during snapshot creation; review again.".into());
        }
        if is_notes {
            let text = fs::read_to_string(&dest).map_err(|e| e.to_string())?;
            if text.trim().is_empty() {
                return Err("Release notes are empty.".into());
            }
            notes = Some(dest);
        } else {
            assets.push(dest);
        }
    }
    let notes_file = notes.ok_or("Missing release-notes.md")?;
    if assets.is_empty() {
        return Err("No release assets found.".into());
    }
    assets.sort();
    Ok(ReleaseBundle {
        notes_file,
        assets,
        _snapshot: snapshot,
    })
}
fn checked_repo(repo: &str) -> Result<(), String> {
    if crate::git::github_identity(&format!("https://github.com/{repo}")).is_none() {
        return Err("Invalid GitHub repository identity.".into());
    }
    Ok(())
}
fn run_gh(repo: &str, args: &[String]) -> Result<String, String> {
    checked_repo(repo)?;
    let mut all = args.to_vec();
    all.extend(["--repo".into(), repo.into()]);
    let refs = all.iter().map(String::as_str).collect::<Vec<_>>();
    let o = crate::process::capture_with_wait("gh", &refs, None)?;
    if o.success {
        Ok(crate::process::safe_text(&o.stdout))
    } else {
        Err(crate::process::failure("gh", &o))
    }
}
fn batches(bundle: &ReleaseBundle) -> Result<Vec<Vec<String>>, String> {
    let mut result = Vec::new();
    let mut batch = Vec::new();
    let mut size = 0;
    for path in &bundle.assets {
        let value = path.to_str().ok_or("Non-UTF-8 snapshot path")?.to_owned();
        if value.len() > 8192 {
            return Err("Snapshot path exceeds safe command-line limit.".into());
        }
        if !batch.is_empty() && (size + value.len() > 12000 || batch.len() >= 32) {
            result.push(std::mem::take(&mut batch));
            size = 0;
        }
        size += value.len() + 1;
        batch.push(value);
    }
    if !batch.is_empty() {
        result.push(batch);
    }
    Ok(result)
}
pub fn create_release(
    repo: &str,
    tag: &str,
    title: &str,
    bundle: &ReleaseBundle,
    kind: ReleaseKind,
    target: &str,
) -> Result<String, String> {
    checked_repo(repo)?;
    validate_tag(tag)?;
    if target.len() != 40 || !target.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Release requires a verified full source commit SHA.".into());
    }
    let batches = batches(bundle)?;
    // Verify the exact commit exists on the selected remote, never implicitly
    // create a tag on a moving default branch.
    let endpoint = format!("repos/{repo}/commits/{target}");
    let o = crate::process::capture("gh", &["api", &endpoint, "--jq", ".sha"], None)?;
    if !o.success || o.stdout.trim() != target {
        return Err("Selected source commit is not verified on the selected GitHub repository. Push it explicitly first.".into());
    }
    let mut args = vec![
        "release".into(),
        "create".into(),
        tag.into(),
        "--title".into(),
        title.into(),
        "--notes-file".into(),
        bundle
            .notes_file
            .to_str()
            .ok_or("Invalid notes path")?
            .into(),
        "--target".into(),
        target.into(),
    ];
    match kind {
        ReleaseKind::Release => {}
        ReleaseKind::Prerelease => args.push("--prerelease".into()),
        ReleaseKind::Draft => args.push("--draft".into()),
    }
    run_gh(repo, &args)?;
    upload_batches(repo,tag,batches).map_err(|e|format!("Release {tag} was created; some assets may already exist. Do not delete or clobber it. Review remote assets before retrying.\n{e}"))
}
fn upload_batches(repo: &str, tag: &str, batches: Vec<Vec<String>>) -> Result<String, String> {
    for (i, batch) in batches.into_iter().enumerate() {
        let mut args = vec!["release".into(), "upload".into(), tag.into()];
        args.extend(batch);
        run_gh(repo,&args).map_err(|e|format!("Upload batch {} failed. Previous batches may be complete; existing assets were never overwritten.\n{e}",i+1))?;
    }
    Ok(format!(
        "Release {tag}: all requested asset batches completed."
    ))
}
pub fn upload_to_existing(repo: &str, tag: &str, bundle: &ReleaseBundle) -> Result<String, String> {
    validate_tag(tag)?;
    upload_batches(repo, tag, batches(bundle)?)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseKind {
    Release,
    Prerelease,
    Draft,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("release-notes.md"), "Notes").unwrap();
        fs::write(d.path().join("app.zip"), b"old").unwrap();
        d
    }
    #[cfg(unix)]
    #[test]
    fn snapshot_is_immutable_from_source() {
        let d = fixture();
        let b = collect_bundle(d.path()).unwrap();
        fs::write(d.path().join("app.zip"), b"new").unwrap();
        assert_eq!(fs::read(&b.assets[0]).unwrap(), b"old");
    }
    #[cfg(unix)]
    #[test]
    fn duplicate_names_rejected() {
        let d = fixture();
        fs::create_dir(d.path().join("nested")).unwrap();
        fs::write(d.path().join("nested/app.zip"), b"x").unwrap();
        assert!(collect_bundle(d.path()).unwrap_err().contains("Duplicate"));
    }
    #[test]
    fn help_tag_rejected_before_cli() {
        assert!(validate_tag("--help").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn hash_filename_rejected() {
        let d = fixture();
        fs::write(d.path().join("a#b.zip"), b"x").unwrap();
        assert!(collect_bundle(d.path()).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn external_notes_link_rejected() {
        let d = fixture();
        fs::remove_file(d.path().join("release-notes.md")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", d.path().join("release-notes.md")).unwrap();
        assert!(collect_bundle(d.path()).is_err());
    }
    #[cfg(not(unix))]
    #[test]
    fn unsupported_folder_upload_is_explicit() {
        let d = fixture();
        assert!(collect_bundle(d.path()).unwrap_err().contains("disabled"));
    }
    #[test]
    fn batches_are_bounded() {
        let d = fixture();
        let mut b = ReleaseBundle {
            notes_file: d.path().join("release-notes.md"),
            assets: Vec::new(),
            _snapshot: d,
        };
        b.assets = (0..20000)
            .map(|i| b._snapshot.path().join(format!("asset-{i:08}.zip")))
            .collect();
        let groups = batches(&b).unwrap();
        assert!(groups.len() > 1);
        assert!(groups
            .iter()
            .all(|g| g.len() <= 32 && g.iter().map(|s| s.len() + 1).sum::<usize>() <= 12000));
    }
}
