use crate::toolchain;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(60);
/// Windows can't start a process with a command line over 32,767 characters.
const MAX_PATHS_LENGTH: usize = 24_000;
const NO_PROJECT_CODE: [&str; 8] = [
    "-c",
    if cfg!(windows) { "core.hooksPath=NUL" } else { "core.hooksPath=/dev/null" },
    "-c",
    "core.fsmonitor=false",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "core.longpaths=true",
];

/// Settings that name a program. A project that arrives as a zip brings its own `.git/config`,
/// so these would run on this computer as soon as Ferry calls Git.
fn runs_a_program(key: &str, value: &str) -> bool {
    let key = key.to_ascii_lowercase();
    let exact = ["core.fsmonitor", "core.sshcommand", "core.gitproxy", "core.pager", "core.editor", "core.askpass", "sequence.editor"];
    if key == "core.fsmonitor" && matches!(value.to_ascii_lowercase().as_str(), "" | "false" | "no" | "off" | "0") {
        return false;
    }
    if key.starts_with("filter.lfs.") && (value.is_empty() || value.starts_with("git-lfs ") || matches!(value, "true" | "false")) {
        return false;
    }
    exact.contains(&key.as_str())
        || (key.starts_with("hook.") && key.ends_with(".command"))
        || key == "diff.external"
        || key == "credential.helper"
        || (key.starts_with("gpg.") && key.ends_with("program"))
        || (key.starts_with("filter.") && [".clean", ".smudge", ".process"].iter().any(|s| key.ends_with(s)))
        || (key.starts_with("diff.") && [".command", ".textconv"].iter().any(|s| key.ends_with(s)))
        || (key.starts_with("merge.") && key.ends_with(".driver"))
        || (key.starts_with("credential.") && key.ends_with(".helper"))
}

fn parts(paths: &[String]) -> Vec<&[String]> {
    let (mut out, mut start, mut length) = (Vec::new(), 0, 0);
    for (at, path) in paths.iter().enumerate() {
        if at > start && length + path.len() > MAX_PATHS_LENGTH {
            out.push(&paths[start..at]);
            (start, length) = (at, 0);
        }
        length += path.len() + 3;
    }
    out.push(&paths[start..]);
    out
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    pub repo: bool,
    pub dirty: Vec<String>,
}

struct Git {
    program: PathBuf,
    root: PathBuf,
}

impl Git {
    /// `Ok(None)` means there's no repository to care about. Any other failure, like a folder Git
    /// refuses to trust, is an error: treating it as "no repository" would skip the checks.
    fn find(root: &Path) -> Result<Option<Git>, String> {
        let Some(program) = toolchain::find_executable("git") else { return Ok(None) };
        let git = Git { program, root: root.to_path_buf() };
        match git.call(&["rev-parse", "--is-inside-work-tree"], &[], true) {
            Ok(out) => Ok((String::from_utf8_lossy(&out).trim() == "true").then_some(git)),
            Err(reason) if reason.contains("not a git repository") => Ok(None),
            Err(reason) => Err(reason),
        }
    }

    /// Project hooks and terminal prompts stay off: a commit made for the user must not run the
    /// project's code or wait for input that can never come.
    /// A long list of paths runs in parts, so `args` must be a command that allows that.
    fn run(&self, args: &[&str], paths: &[String]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for part in parts(paths) {
            out.extend(self.call(args, part, false)?);
        }
        Ok(out)
    }

    /// A commit can't run in parts. Only a list too long for the command line needs Git 2.25.
    fn commit_paths(&self, message: &str, paths: &[String]) -> Result<(), String> {
        if parts(paths).len() == 1 {
            return self.run(&["commit", "-m", message, "--"], paths).map(|_| ());
        }
        let mut list = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut list, paths.join("\0").as_bytes()).map_err(|e| e.to_string())?;
        let from = format!("--pathspec-from-file={}", list.path().display());
        self.run(&["commit", "-m", message, &from, "--pathspec-file-nul"], &[]).map(|_| ())
    }

    /// Paths are taken literally, so `a[1].php` can't match `a1.php` too.
    fn call(&self, args: &[&str], paths: &[String], untranslated: bool) -> Result<Vec<u8>, String> {
        let mut cmd = toolchain::command(&self.program);
        cmd.arg("-C").arg(&self.root).args(NO_PROJECT_CODE).args(args).args(paths);
        cmd.env("GIT_TERMINAL_PROMPT", "0").env("GIT_LITERAL_PATHSPECS", "1");
        if untranslated {
            cmd.env("LC_ALL", "C");
        }
        let done = toolchain::run_process(cmd, &self.program, &AtomicBool::new(false), TIMEOUT).map_err(|e| e.to_string())?;
        if done.status.is_some_and(|s| s.success()) {
            return Ok(done.stdout);
        }
        let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
        Err(if reason.is_empty() { crate::t!("Git yanıt vermedi.", "Git didn't respond.") } else { reason })
    }

    fn ok(&self, args: &[&str]) -> bool {
        self.run(args, &[]).is_ok()
    }

    /// Reading the settings runs nothing. `--show-scope` keeps what the user set up themself out of it.
    fn refuse_project_programs(&self) -> Result<(), String> {
        let out = self.run(&["config", "--includes", "--show-scope", "--list", "-z"], &[])?;
        let mut fields = out.split(|b| *b == 0).filter(|f| !f.is_empty());
        while let (Some(scope), Some(entry)) = (fields.next(), fields.next()) {
            if !matches!(scope, b"local" | b"worktree") {
                continue;
            }
            let entry = String::from_utf8_lossy(entry);
            let (key, value) = entry.split_once('\n').unwrap_or((&entry, ""));
            if runs_a_program(key, value) {
                return Err(crate::t!(
                    "Bu projenin Git ayarı ({key}) bir program çalıştırıyor. Güvenlik için Ferry bu depoda Git kullanmıyor; ayarı inceleyip kaldırın.",
                    "This project's Git setting ({key}) runs a program. For safety, Ferry won't use Git in this repository. Review the setting and remove it."
                ));
            }
        }
        Ok(())
    }

    fn text(&self, args: &[&str]) -> Option<String> {
        self.run(args, &[]).ok().map(|out| String::from_utf8_lossy(&out).trim().to_string())
    }
}

pub fn porcelain_paths(out: &[u8], prefix: &str) -> Vec<String> {
    let mut entries = out.split(|b| *b == 0).filter(|e| !e.is_empty());
    let mut paths = Vec::new();
    while let Some(entry) = entries.next() {
        let Some(path) = entry.get(3..) else { continue };
        if matches!(entry[0], b'R' | b'C') {
            entries.next();
        }
        let path = String::from_utf8_lossy(path);
        paths.push(path.strip_prefix(prefix).unwrap_or(&path).to_string());
    }
    paths
}

/// With no files, reports everything under the project.
pub fn status(root: &Path, files: &[String]) -> Result<Status, String> {
    let Some(git) = Git::find(root)? else { return Ok(Status::default()) };
    git.refuse_project_programs()?;
    let prefix = git.text(&["rev-parse", "--show-prefix"]).unwrap_or_default();
    let everything = [".".to_string()];
    let paths = if files.is_empty() { &everything[..] } else { files };
    let out = git.run(&["status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignore-submodules=dirty", "--"], paths)?;
    Ok(Status { repo: true, dirty: porcelain_paths(&out, &prefix) })
}

/// `git blame --line-porcelain` for one file.
pub fn blame(root: &Path, file: &str) -> Result<Vec<u8>, String> {
    let git = Git::find(root)?.ok_or_else(|| crate::t!("Bu proje bir Git deposu değil.", "This project isn't a Git repository."))?;
    git.refuse_project_programs()?;
    git.run(&["blame", "--no-textconv", "--line-porcelain", "--"], &[file.to_string()])
}

/// Ignored files and files inside a submodule can't go into the project's commit. This has to be
/// known before anything is written.
pub fn check_committable(root: &Path, files: &[String]) -> Result<(), String> {
    let Some(git) = Git::find(root)? else { return Ok(()) };
    git.refuse_project_programs()?;
    git.run(&["add", "--dry-run", "--"], files).map(|_| ()).map_err(|reason| {
        crate::t!(
            "Bu dosyalar ayrı commit'e alınamıyor: {reason}\nAyrı dal seçeneğini kapatıp yeniden deneyin.",
            "These files can't go into a separate commit: {reason}\nTurn the separate branch off and try again."
        )
    })
}

/// A branch made for one batch of changes, and the place to return to if the batch fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub branch: String,
    /// A full `refs/heads/` name, or a commit if HEAD was detached.
    previous: String,
}

/// The working tree stays as it is, so a preview made on the old branch is still valid.
pub fn start_batch(root: &Path, base: &str) -> Result<Batch, String> {
    let git = Git::find(root)?.ok_or_else(|| crate::t!("Bu proje bir Git deposu değil.", "This project isn't a Git repository."))?;
    git.refuse_project_programs()?;
    if !git.ok(&["rev-parse", "--verify", "HEAD"]) {
        return Err(crate::t!(
            "Depoda henüz commit yok, bu yüzden ayrı dal açılamadı.",
            "The repository has no commits yet, so a separate branch can't be created."
        ));
    }
    let previous = git.text(&["symbolic-ref", "-q", "HEAD"]).or_else(|| git.text(&["rev-parse", "HEAD"])).unwrap_or_default();
    let taken = |name: &str| git.ok(&["rev-parse", "--verify", "-q", &format!("refs/heads/{name}")]);
    let branch = (1..1000).map(|n| if n == 1 { base.to_string() } else { format!("{base}-{n}") }).find(|name| !taken(name));
    let branch = branch.ok_or_else(|| crate::t!("Boş bir dal adı bulunamadı.", "No free branch name was found."))?;
    git.run(&["switch", "-c", &branch], &[])?;
    Ok(Batch { branch, previous })
}

/// Written files stay as they are. Returns whether the repository is back where it was.
pub fn abort_batch(root: &Path, batch: &Batch) -> bool {
    let Ok(Some(git)) = Git::find(root) else { return false };
    let back = match batch.previous.strip_prefix("refs/heads/") {
        Some(name) => git.ok(&["switch", name]),
        None => git.ok(&["switch", "--detach", &batch.previous]),
    };
    back && git.ok(&["branch", "-D", &batch.branch])
}

/// Only these files go into the commit, whatever else is staged. Nothing stays staged if it
/// fails. Returns the short hash.
pub fn commit(root: &Path, files: &[String], message: &str) -> Result<String, String> {
    let git = Git::find(root)?.ok_or_else(|| crate::t!("Git bulunamadı.", "Git wasn't found."))?;
    git.refuse_project_programs()?;
    if let Err(e) = git.run(&["add", "--"], files).and_then(|_| git.commit_paths(message, files)) {
        let _ = git.run(&["reset", "-q", "--"], files);
        return Err(e);
    }
    git.text(&["rev-parse", "--short", "HEAD"]).ok_or_else(|| crate::t!("Git yanıt vermedi.", "Git didn't respond."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> Option<(tempfile::TempDir, Git)> {
        let program = toolchain::find_executable("git")?;
        let dir = tempfile::tempdir().unwrap();
        let git = Git { program, root: dir.path().to_path_buf() };
        git.run(&["init", "-q", "-b", "main"], &[]).unwrap();
        git.run(&["config", "user.email", "t@example.com"], &[]).unwrap();
        git.run(&["config", "user.name", "T"], &[]).unwrap();
        std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b\n").unwrap();
        git.run(&["add", "."], &[]).unwrap();
        git.run(&["commit", "-qm", "first"], &[]).unwrap();
        Some((dir, git))
    }

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[cfg(unix)]
    #[test]
    fn a_submodule_cant_run_its_own_filter_during_status() {
        let (Some((dir, git)), Some((inner_dir, _))) = (repo(), repo()) else { return };
        let root = dir.path();
        let source = inner_dir.path().display().to_string();
        git.run(&["-c", "protocol.file.allow=always", "submodule", "add", "-q", &source, "sub"], &[]).unwrap();
        git.run(&["commit", "-qm", "sub"], &[]).unwrap();
        let inner = Git { program: git.program.clone(), root: root.join("sub") };
        let marker = root.join("ran");
        inner.run(&["config", "filter.x.clean", &format!("touch {}; cat", marker.display())], &[]).unwrap();
        std::fs::write(root.join("sub/.gitattributes"), "a.txt filter=x\n").unwrap();
        std::fs::write(root.join("sub/a.txt"), "c\n").unwrap();
        status(root, &[]).unwrap();
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[test]
    fn blame_refuses_a_project_that_would_run_a_program() {
        let Some((dir, git)) = repo() else { return };
        let root = dir.path();
        assert!(String::from_utf8_lossy(&blame(root, "a.txt").unwrap()).contains("author T"));
        let marker = root.join("ran");
        git.run(&["config", "diff.x.textconv", &format!("touch {}; cat", marker.display())], &[]).unwrap();
        std::fs::write(root.join(".gitattributes"), "a.txt diff=x\n").unwrap();
        assert!(blame(root, "a.txt").is_err());
        assert!(!marker.exists());
    }

    #[test]
    fn a_long_list_of_files_fits_the_command_line() {
        let Some((dir, git)) = repo() else { return };
        let root = dir.path();
        let names: Vec<String> = (0..1200).map(|n| format!("a-file-with-a-long-enough-name-{n:04}.php")).collect();
        for name in &names {
            std::fs::write(root.join(name), "<?php\n").unwrap();
        }
        assert!(parts(&names).len() > 1);
        std::fs::write(root.join("b.txt"), "someone else's work\n").unwrap();
        git.run(&["add", "b.txt"], &[]).unwrap();
        assert_eq!(status(root, &names).unwrap().dirty.len(), names.len());
        check_committable(root, &names).unwrap();
        commit(root, &names, "Many").unwrap();
        let committed = git.text(&["show", "--format=", "--name-only", "HEAD"]).unwrap();
        assert_eq!(committed.lines().count(), names.len());
        assert!(!committed.contains("b.txt"));
        assert_eq!(status(root, &[]).unwrap().dirty, ["b.txt"]);
    }

    #[test]
    fn a_batch_gets_its_own_branch_and_only_its_files() {
        let Some((dir, git)) = repo() else { return };
        let root = dir.path();
        let first = start_batch(root, "ferry/php-8.4").unwrap();
        assert_eq!(first.branch, "ferry/php-8.4");
        std::fs::write(root.join("a.txt"), "changed\n").unwrap();
        std::fs::write(root.join("b.txt"), "someone else's work\n").unwrap();
        let hash = commit(root, &files(&["a.txt"]), "Batch").unwrap();
        assert_eq!(git.text(&["rev-parse", "--short", "HEAD"]).unwrap(), hash);
        assert_eq!(git.text(&["show", "--format=%s", "--name-only", "HEAD"]).unwrap(), "Batch\n\na.txt");
        assert_eq!(status(root, &[]).unwrap().dirty, ["b.txt"]);
        git.run(&["switch", "main"], &[]).unwrap();
        assert_eq!(start_batch(root, "ferry/php-8.4").unwrap().branch, "ferry/php-8.4-2");
    }

    #[test]
    fn a_failed_batch_leaves_the_repository_where_it_was() {
        let Some((dir, git)) = repo() else { return };
        let batch = start_batch(dir.path(), "ferry/php-8.4").unwrap();
        assert!(abort_batch(dir.path(), &batch));
        assert_eq!(git.text(&["symbolic-ref", "--short", "HEAD"]).unwrap(), "main");
        assert!(!git.ok(&["rev-parse", "--verify", "-q", "refs/heads/ferry/php-8.4"]));
    }

    #[cfg(unix)]
    #[test]
    fn project_hooks_do_not_run() {
        let Some((dir, _)) = repo() else { return };
        let root = dir.path();
        let hook = root.join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\ntouch ran\nexit 1\n").unwrap();
        std::process::Command::new("chmod").arg("755").arg(&hook).status().unwrap();
        std::fs::write(root.join("a.txt"), "changed\n").unwrap();
        commit(root, &files(&["a.txt"]), "Batch").unwrap();
        assert!(!root.join("ran").exists());
    }

    #[cfg(unix)]
    #[test]
    fn programs_named_in_the_project_settings_do_not_run() {
        for (key, value) in [
            ("core.fsmonitor", "./evil.sh"),
            ("filter.x.clean", "./evil.sh"),
            ("gpg.program", "./evil.sh"),
            ("diff.x.textconv", "./evil.sh"),
        ] {
            let Some((dir, git)) = repo() else { return };
            let root = dir.path();
            std::fs::write(root.join("evil.sh"), "#!/bin/sh\ntouch ran\n").unwrap();
            std::process::Command::new("chmod").arg("755").arg(root.join("evil.sh")).status().unwrap();
            git.run(&["config", key, value], &[]).unwrap();
            std::fs::write(root.join(".gitattributes"), "* filter=x diff=x\n").unwrap();
            let refused = status(root, &[]).unwrap_err();
            assert!(refused.contains(key), "{refused}");
            assert!(start_batch(root, "ferry/x").is_err());
            assert!(commit(root, &files(&["a.txt"]), "x").is_err());
            assert!(!root.join("ran").exists(), "{key}");
        }
    }

    #[test]
    fn ordinary_settings_are_fine() {
        assert!(!runs_a_program("core.fsmonitor", "false"));
        assert!(!runs_a_program("user.name", "x"));
        assert!(!runs_a_program("filter.lfs.clean", "git-lfs clean -- %f"));
        assert!(runs_a_program("filter.lfs.clean", "./evil.sh"));
        assert!(runs_a_program("hook.audit.command", "./evil.sh"));
        assert!(!runs_a_program("hook.audit.event", "pre-commit"));
    }

    #[test]
    fn nothing_is_reported_outside_a_repository() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(status(dir.path(), &[]).unwrap(), Status::default());
        assert!(start_batch(dir.path(), "x").is_err());
    }

    #[test]
    fn a_repository_git_refuses_is_not_treated_as_no_repository() {
        let Some((dir, git)) = repo() else { return };
        git.run(&["config", "core.repositoryformatversion", "99"], &[]).unwrap();
        assert!(status(dir.path(), &[]).is_err());
    }

    #[test]
    fn special_characters_in_a_path_are_literal() {
        let Some((dir, git)) = repo() else { return };
        let root = dir.path();
        std::fs::write(root.join("a[1].php"), "x\n").unwrap();
        std::fs::write(root.join("a1.php"), "y\n").unwrap();
        git.run(&["add", "."], &[]).unwrap();
        git.run(&["commit", "-qm", "both"], &[]).unwrap();
        std::fs::write(root.join("a[1].php"), "x2\n").unwrap();
        std::fs::write(root.join("a1.php"), "y2\n").unwrap();
        assert_eq!(status(root, &files(&["a[1].php"])).unwrap().dirty, ["a[1].php"]);
        commit(root, &files(&["a[1].php"]), "One").unwrap();
        assert_eq!(git.text(&["show", "--format=", "--name-only", "HEAD"]).unwrap(), "a[1].php");
        assert_eq!(status(root, &[]).unwrap().dirty, ["a1.php"]);
    }

    #[test]
    fn ignored_files_are_found_before_writing() {
        let Some((dir, _)) = repo() else { return };
        let root = dir.path();
        std::fs::write(root.join(".gitignore"), "skip.php\n").unwrap();
        std::fs::write(root.join("skip.php"), "x\n").unwrap();
        assert!(check_committable(root, &files(&["skip.php"])).is_err());
        assert!(check_committable(root, &files(&["a.txt"])).is_ok());
    }

    #[test]
    fn a_failed_commit_leaves_nothing_staged() {
        let Some((dir, git)) = repo() else { return };
        let root = dir.path();
        std::fs::write(root.join("a.txt"), "changed\n").unwrap();
        git.run(&["config", "commit.cleanup", "nonsense"], &[]).unwrap();
        assert!(commit(root, &files(&["a.txt"]), "Batch").is_err());
        assert_eq!(git.text(&["diff", "--cached", "--name-only"]).unwrap(), "");
        assert_eq!(status(root, &[]).unwrap().dirty, ["a.txt"]);
    }

    #[test]
    fn a_batch_returns_to_a_branch_named_like_a_tag() {
        let Some((dir, git)) = repo() else { return };
        git.run(&["tag", "main"], &[]).unwrap();
        let batch = start_batch(dir.path(), "ferry/php-8.4").unwrap();
        assert!(abort_batch(dir.path(), &batch));
        assert_eq!(git.text(&["symbolic-ref", "HEAD"]).unwrap(), "refs/heads/main");
        assert!(!git.ok(&["rev-parse", "--verify", "-q", "refs/heads/ferry/php-8.4"]));
    }

    #[test]
    fn a_batch_returns_to_a_detached_head() {
        let Some((dir, git)) = repo() else { return };
        let commit = git.text(&["rev-parse", "HEAD"]).unwrap();
        git.run(&["switch", "--detach", "HEAD"], &[]).unwrap();
        let batch = start_batch(dir.path(), "ferry/php-8.4").unwrap();
        assert!(abort_batch(dir.path(), &batch));
        assert_eq!(git.text(&["rev-parse", "HEAD"]).unwrap(), commit);
        assert!(!git.ok(&["symbolic-ref", "-q", "HEAD"]));
    }
}
