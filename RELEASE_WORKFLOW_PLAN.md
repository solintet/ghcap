# GitHub Release folder workflow (implemented in v0.26.0; Enter-to-continue confirmation and wait messages added in v0.0.29.7)

Select **GitHub Release (folder upload)** from a repository menu and enter a staging directory. Relative paths are created under the configured local repository folder; the default is `release-staging`. ghcap creates the folder if it does not exist. Put a non-empty `release-notes.md` at its root, then put all release files in the folder.

All other regular files beneath the directory are uploaded recursively as assets. Symlinks and `.git`, `.svn`, and `__MACOSX` contents are skipped. Duplicate asset basenames are rejected because GitHub Release asset names must be unique. The staging directory is inside the working repository by default and may appear as untracked content in Git; it is not itself uploaded as an archive.

Before running `gh`, ghcap displays the repository, tag, title, release type, notes path, and complete asset list. Press Enter to continue; Ctrl-D cancels. Long-running GitHub checks and uploads display `Running... Please wait.` every second. The workflow uses the installed official GitHub CLI (`gh release create` / `gh release upload`), not a custom GitHub API implementation.

- New release: creates a Release, Pre-release, or Draft with `--notes-file`.
- Existing release: asks before uploading assets. It does not alter title/notes/type and never uses `--clobber`; duplicate remote asset names fail safely.
- Tag and title are entered interactively. A new tag targets the repository's default branch unless the installed `gh` CLI's normal defaults say otherwise.
- Build/export is not automated in this version: place built files in the staging folder before creating the release.
