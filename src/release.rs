use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ReleaseBundle {
    pub notes_file: PathBuf,
    pub assets: Vec<PathBuf>,
}

/// Reads a staging directory. `release-notes.md` at the root is required;
/// all other regular files, including files in subdirectories, become assets.
/// Symlinks are intentionally skipped so a staging folder cannot upload files
/// from outside the selected directory by accident.
pub fn collect_bundle(directory: &Path) -> Result<ReleaseBundle, String> {
    if !directory.is_dir() {
        return Err(format!("Not a directory: {}", directory.display()));
    }
    let note_candidates = ["release-notes.md", "RELEASE_NOTES.md", "Release-Notes.md"]
        .iter()
        .map(|name| directory.join(name))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    if note_candidates.len() > 1 {
        return Err("Multiple release note files found. Keep only one of release-notes.md, RELEASE_NOTES.md, or Release-Notes.md.".into());
    }
    let notes_file = note_candidates
        .into_iter()
        .next()
        .ok_or_else(|| format!("Missing release-notes.md in {}", directory.display()))?;
    let notes = fs::read_to_string(&notes_file).map_err(|error| {
        format!(
            "Cannot read release notes {}: {error}",
            notes_file.display()
        )
    })?;
    if notes.trim().is_empty() {
        return Err("Release notes are empty. Add content to release-notes.md first.".into());
    }

    let mut assets = Vec::new();
    collect_files(directory, directory, &notes_file, &mut assets)?;
    if assets.is_empty() {
        return Err(
            "No release assets found. Put the files to upload in the staging folder.".into(),
        );
    }
    let mut names = HashSet::new();
    for asset in &assets {
        let name = asset
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("Non-UTF-8 asset filename: {}", asset.display()))?;
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!("Duplicate asset filename '{name}'. GitHub Release assets must have unique names; rename one of the files."));
        }
    }
    assets.sort();
    Ok(ReleaseBundle { notes_file, assets })
}

fn collect_files(
    root: &Path,
    current: &Path,
    notes_file: &Path,
    assets: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let is_metadata = |path: &Path| {
        path.strip_prefix(root)
            .unwrap_or(path)
            .components()
            .any(|component| {
                matches!(
                    component.as_os_str().to_str(),
                    Some(".git" | ".svn" | "__MACOSX")
                )
            })
    };

    for entry in WalkDir::new(current)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.path() == current || !is_metadata(entry.path()))
    {
        let entry = entry.map_err(|error| {
            let path = error
                .path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| current.display().to_string());
            format!("Cannot inspect release path {path}: {error}")
        })?;
        let path = entry.path();
        if path == current || path == notes_file {
            continue;
        }
        if entry.file_type().is_file() {
            assets.push(path.to_path_buf());
        }
    }
    Ok(())
}

pub fn create_release(
    repo: &str,
    tag: &str,
    title: &str,
    bundle: &ReleaseBundle,
    kind: ReleaseKind,
) -> Result<String, String> {
    let mut args = vec!["release".to_string(), "create".to_string(), tag.to_string()];
    for asset in &bundle.assets {
        args.push(asset.to_string_lossy().into_owned());
    }
    args.push("--title".into());
    args.push(title.into());
    args.push("--notes-file".into());
    args.push(bundle.notes_file.to_string_lossy().into_owned());
    match kind {
        ReleaseKind::Release => {}
        ReleaseKind::Prerelease => args.push("--prerelease".into()),
        ReleaseKind::Draft => args.push("--draft".into()),
    }
    run_gh(repo, &args)
}

pub fn upload_to_existing(repo: &str, tag: &str, bundle: &ReleaseBundle) -> Result<String, String> {
    let mut args = vec!["release".to_string(), "upload".to_string(), tag.to_string()];
    for asset in &bundle.assets {
        args.push(asset.to_string_lossy().into_owned());
    }
    // Do not use --clobber: existing assets must never be overwritten silently.
    run_gh(repo, &args)
}

fn run_gh(repo: &str, args: &[String]) -> Result<String, String> {
    let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = crate::process::capture_with_wait("gh", &borrowed, Some(Path::new(repo)))?;
    if output.success {
        let stdout = output.stdout.trim();
        let stderr = output.stderr.trim();
        Ok(if !stdout.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        })
    } else {
        Err(crate::process::failure("gh", &output))
    }
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
    use std::fs;

    #[test]
    fn bundle_uses_notes_and_collects_nested_assets() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("release-notes.md"), "Release notes\n").unwrap();
        fs::write(dir.path().join("app.zip"), b"zip").unwrap();
        fs::create_dir(dir.path().join("packages")).unwrap();
        fs::write(dir.path().join("packages").join("app.deb"), b"deb").unwrap();
        let bundle = collect_bundle(dir.path()).unwrap();
        assert_eq!(bundle.assets.len(), 2);
        assert!(bundle
            .assets
            .iter()
            .any(|p| p.file_name().unwrap() == "app.zip"));
        assert!(bundle
            .assets
            .iter()
            .any(|p| p.file_name().unwrap() == "app.deb"));
    }

    #[test]
    fn bundle_rejects_duplicate_asset_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("release-notes.md"), "Release notes\n").unwrap();
        fs::write(dir.path().join("app.zip"), b"one").unwrap();
        fs::create_dir(dir.path().join("other")).unwrap();
        fs::write(dir.path().join("other").join("app.zip"), b"two").unwrap();
        assert!(collect_bundle(dir.path())
            .unwrap_err()
            .contains("Duplicate asset filename"));
    }
}
