use super::*;
use tempfile::TempDir;

fn repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    command(dir.path(), &["init", "-b", "main"], None).unwrap();
    command(dir.path(), &["config", "user.name", "GitPane Test"], None).unwrap();
    command(
        dir.path(),
        &["config", "user.email", "test@gitpane.invalid"],
        None,
    )
    .unwrap();
    command(dir.path(), &["config", "commit.gpgsign", "false"], None).unwrap();
    command(dir.path(), &["config", "core.autocrlf", "false"], None).unwrap();
    std::fs::create_dir(dir.path().join(".hooks")).unwrap();
    command(
        dir.path(),
        &[
            "config",
            "core.hooksPath",
            dir.path().join(".hooks").to_str().unwrap(),
        ],
        None,
    )
    .unwrap();
    dir
}

fn write(root: &Path, name: &str, content: &str) {
    std::fs::write(root.join(name), content).unwrap();
}

fn stage(root: &Path, names: &[&str]) {
    mutate(
        root,
        Mutation::Stage {
            paths: names.iter().map(|s| s.to_string()).collect(),
        },
    )
    .unwrap();
}

fn commit(root: &Path) {
    mutate(
        root,
        Mutation::Commit {
            message: "test: initial state".into(),
        },
    )
    .unwrap();
}

#[test]
fn parse_renames_spaces_unicode_and_conflicts() {
    let changes = parse_status(
        "R  新 name.ts\0old name.ts\0 M file [1].ts\0UU conflict.ts\0?? fresh.ts\0".as_bytes(),
    )
    .unwrap();
    assert_eq!(changes.len(), 4);
    assert_eq!(changes[0].original_path.as_deref(), Some("old name.ts"));
    assert_eq!(changes[0].path, "新 name.ts");
    assert!(changes[2].conflict);
}

#[test]
fn unborn_unstage_preserves_worktree_and_literal_paths() {
    let dir = repo();
    let root = dir.path();
    write(root, "file [1].txt", "hello\n");
    write(root, "file 1.txt", "other\n");
    stage(root, &["file [1].txt"]);
    let staged = files(root).unwrap();
    assert_eq!(staged.iter().filter(|f| f.index == 'A').count(), 1);
    mutate(
        root,
        Mutation::Unstage {
            paths: vec!["file [1].txt".into()],
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("file [1].txt")).unwrap(),
        "hello\n"
    );
    assert!(files(root).unwrap().iter().all(|f| f.index == '?'));
}

#[test]
fn commit_includes_only_staged_content_and_history() {
    let dir = repo();
    let root = dir.path();
    write(root, "a.txt", "first\n");
    stage(root, &["a.txt"]);
    write(root, "a.txt", "second\n");
    commit(root);
    assert_eq!(text(root, &["show", "HEAD:a.txt"]).unwrap(), "first\n");
    let state = snapshot(root).unwrap();
    assert_eq!(state.branch, "main");
    assert_eq!(state.files[0].worktree, 'M');
    let log = history(root, 0, false).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].subject, "test: initial state");
    assert!(history(root, 60, false).unwrap().is_empty());
    assert!(commit_patch(root, &log[0].oid)
        .unwrap()
        .patch
        .contains("+first"));
}

#[test]
fn stage_one_hunk_and_reject_stale_patch() {
    let dir = repo();
    let root = dir.path();
    let initial = (1..=30).map(|i| format!("line {i}\n")).collect::<String>();
    write(root, "a.txt", &initial);
    stage(root, &["a.txt"]);
    commit(root);
    let changed = initial
        .replace("line 2\n", "second line\n")
        .replace("line 27\n", "last change\n");
    write(root, "a.txt", &changed);
    let patch = diff(root, "a.txt", false).unwrap();
    assert!(patch.can_stage_hunks);
    mutate(
        root,
        Mutation::StageHunk {
            path: "a.txt".into(),
            expected: patch.patch.clone(),
            hunk: 0,
        },
    )
    .unwrap();
    let cached = text(root, &["show", ":a.txt"]).unwrap();
    assert!(cached.contains("second line"));
    assert!(!cached.contains("last change"));
    assert!(mutate(
        root,
        Mutation::StageHunk {
            path: "a.txt".into(),
            expected: patch.patch,
            hunk: 1
        }
    )
    .is_err());
}

#[test]
fn unstage_rename_preserves_new_file() {
    let dir = repo();
    let root = dir.path();
    write(root, "old.txt", "content\n");
    stage(root, &["old.txt"]);
    commit(root);
    command(root, &["mv", "old.txt", "new.txt"], None).unwrap();
    mutate(
        root,
        Mutation::Unstage {
            paths: vec!["new.txt".into()],
        },
    )
    .unwrap();
    assert!(root.join("new.txt").exists());
    assert!(!root.join("old.txt").exists());
    assert!(command(root, &["diff", "--cached", "--quiet"], None).is_ok());
}

#[test]
fn branch_switch_and_validation() {
    let dir = repo();
    let root = dir.path();
    write(root, "a.txt", "hello\n");
    stage(root, &["a.txt"]);
    commit(root);
    mutate(
        root,
        Mutation::SwitchBranch {
            name: "feature/test".into(),
            create: true,
        },
    )
    .unwrap();
    assert_eq!(snapshot(root).unwrap().branch, "feature/test");
    assert_eq!(branches(root).unwrap().len(), 2);
    assert!(mutate(
        root,
        Mutation::SwitchBranch {
            name: "--detach".into(),
            create: false
        }
    )
    .is_err());
    assert!(diff(root, "../outside.txt", false).is_err());
}

#[test]
fn binary_and_large_files_do_not_render_as_text() {
    let dir = repo();
    let root = dir.path();
    std::fs::write(root.join("binary.bin"), [0, 1, 2, 3]).unwrap();
    assert!(diff(root, "binary.bin", false).unwrap().binary);
    std::fs::write(root.join("large.txt"), vec![b'a'; 1024 * 1024 + 1]).unwrap();
    assert!(diff(root, "large.txt", false).unwrap().truncated);
}

#[test]
fn empty_commit_is_rejected() {
    let dir = repo();
    assert!(mutate(
        dir.path(),
        Mutation::Commit {
            message: String::new()
        }
    )
    .is_err());
    assert!(mutate(
        dir.path(),
        Mutation::Commit {
            message: "nothing staged".into()
        }
    )
    .is_err());
}

#[test]
fn stage_modified_rename_does_not_address_deleted_source() {
    let dir = repo();
    let root = dir.path();
    write(root, "old.txt", "one\ntwo\nthree\nfour\n");
    stage(root, &["old.txt"]);
    commit(root);
    command(root, &["mv", "old.txt", "new.txt"], None).unwrap();
    write(root, "new.txt", "one\ntwo\nthree\nfour\nfive\n");
    stage(root, &["new.txt"]);
    assert_eq!(
        text(root, &["show", ":new.txt"]).unwrap(),
        "one\ntwo\nthree\nfour\nfive\n"
    );
}

#[test]
fn linked_worktree_is_supported() {
    let dir = repo();
    let root = dir.path();
    write(root, "a.txt", "hello\n");
    stage(root, &["a.txt"]);
    commit(root);
    let worktree = tempfile::tempdir().unwrap();
    let workpath = worktree.path().join("linked");
    command(
        root,
        &[
            "worktree",
            "add",
            "-b",
            "linked",
            workpath.to_str().unwrap(),
        ],
        None,
    )
    .unwrap();
    let resolved = open(workpath.to_str().unwrap()).unwrap();
    assert_eq!(snapshot(&resolved).unwrap().branch, "linked");
    write(&resolved, "a.txt", "changed\n");
    assert!(diff(&resolved, "a.txt", false)
        .unwrap()
        .patch
        .contains("+changed"));
    assert!(git_dir(&resolved).unwrap().is_dir());
}

#[test]
fn conflicts_block_commit_until_explicitly_staged() {
    let dir = repo();
    let root = dir.path();
    write(root, "a.txt", "base\n");
    stage(root, &["a.txt"]);
    commit(root);
    command(root, &["switch", "-c", "other"], None).unwrap();
    write(root, "a.txt", "other\n");
    stage(root, &["a.txt"]);
    commit(root);
    command(root, &["switch", "main"], None).unwrap();
    write(root, "a.txt", "main\n");
    stage(root, &["a.txt"]);
    commit(root);
    assert!(command(root, &["merge", "other"], None).is_err());
    assert!(snapshot(root).unwrap().files[0].conflict);
    assert!(mutate(
        root,
        Mutation::Commit {
            message: "must fail".into()
        }
    )
    .is_err());
    write(root, "a.txt", "resolved\n");
    stage(root, &["a.txt"]);
    commit(root);
    assert!(!snapshot(root).unwrap().merging);
    let graph = history(root, 0, true).unwrap();
    assert_eq!(graph[0].parents.len(), 2);
    assert!(commit_patch(root, &graph[0].oid)
        .unwrap()
        .patch
        .contains("resolved"));
}

#[test]
fn history_can_include_non_current_branches() {
    let dir = repo();
    let root = dir.path();
    write(root, "a.txt", "base\n");
    stage(root, &["a.txt"]);
    commit(root);
    command(root, &["switch", "-c", "side"], None).unwrap();
    write(root, "side.txt", "side\n");
    stage(root, &["side.txt"]);
    commit(root);
    command(root, &["switch", "main"], None).unwrap();
    let current = history(root, 0, false).unwrap();
    let all = history(root, 0, true).unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|entry| entry.refs.contains("side")));
    assert!(all
        .iter()
        .any(|entry| entry.parents.contains(&current[0].oid)));
}

#[test]
fn git_log_shows_native_graph_and_respects_scope_and_limit() {
    let dir = repo();
    let root = dir.path();
    assert!(git_log(root, 100, false).unwrap().output.is_empty());
    write(root, "base.txt", "base\n");
    stage(root, &["base.txt"]);
    commit(root);
    command(root, &["switch", "-c", "side"], None).unwrap();
    write(root, "side.txt", "side\n");
    stage(root, &["side.txt"]);
    commit(root);
    command(root, &["switch", "main"], None).unwrap();

    let current = git_log(root, 100, false).unwrap();
    assert!(current.output.contains("test: initial state"));
    assert!(!current.output.contains("side"));
    assert!(current.output.starts_with('*'));
    let all = git_log(root, 1, true).unwrap();
    assert!(all.has_more);
    assert!(all.output.contains("side"));
    let expanded = git_log(root, 100, true).unwrap();
    assert!(!expanded.has_more);
    assert!(expanded.output.contains("main"));
    assert!(expanded.output.contains("side"));
}

#[test]
fn local_remote_configuration_and_publish_round_trip() {
    let dir = repo();
    let root = dir.path();
    write(root, "a.txt", "base\n");
    stage(root, &["a.txt"]);
    commit(root);
    let remote = tempfile::tempdir().unwrap();
    command(remote.path(), &["init", "--bare"], None).unwrap();
    let address = remote.path().to_str().unwrap().to_string();
    mutate(
        root,
        Mutation::AddRemote {
            name: "origin".into(),
            fetch_url: address.clone(),
            push_url: None,
        },
    )
    .unwrap();
    assert_eq!(remotes(root).unwrap()[0].fetch_url, address);
    mutate(
        root,
        Mutation::PublishBranch {
            remote: "origin".into(),
        },
    )
    .unwrap();
    assert_eq!(
        snapshot(root).unwrap().upstream.as_deref(),
        Some("origin/main")
    );
    mutate(
        root,
        Mutation::Remote {
            operation: "fetch".into(),
            remote: Some("origin".into()),
        },
    )
    .unwrap();

    let alternative = tempfile::tempdir().unwrap();
    command(alternative.path(), &["init", "--bare"], None).unwrap();
    let alternative_address = alternative.path().to_str().unwrap().to_string();
    mutate(
        root,
        Mutation::SetRemote {
            name: "origin".into(),
            fetch_url: alternative_address.clone(),
            push_url: Some(address.clone()),
        },
    )
    .unwrap();
    assert_eq!(remotes(root).unwrap()[0].fetch_url, alternative_address);
    assert_eq!(
        remotes(root).unwrap()[0].push_url.as_deref(),
        Some(address.as_str())
    );
    mutate(
        root,
        Mutation::RenameRemote {
            old_name: "origin".into(),
            new_name: "backup".into(),
        },
    )
    .unwrap();
    assert_eq!(remotes(root).unwrap()[0].name, "backup");
    mutate(
        root,
        Mutation::SetRemote {
            name: "backup".into(),
            fetch_url: address,
            push_url: None,
        },
    )
    .unwrap();
    assert!(remotes(root).unwrap()[0].push_url.is_none());
    mutate(
        root,
        Mutation::RemoveRemote {
            name: "backup".into(),
        },
    )
    .unwrap();
    assert!(remotes(root).unwrap().is_empty());
}

#[test]
fn remote_names_urls_and_multiple_push_urls_are_guarded() {
    let dir = repo();
    let root = dir.path();
    assert!(mutate(
        root,
        Mutation::AddRemote {
            name: "--help".into(),
            fetch_url: "https://example.com/repo".into(),
            push_url: None
        }
    )
    .is_err());
    assert!(mutate(
        root,
        Mutation::AddRemote {
            name: "origin".into(),
            fetch_url: "\n[core]\n".into(),
            push_url: None
        }
    )
    .is_err());
    mutate(
        root,
        Mutation::AddRemote {
            name: "origin".into(),
            fetch_url: "https://example.com/repo".into(),
            push_url: None,
        },
    )
    .unwrap();
    command(
        root,
        &["config", "--add", "remote.origin.pushurl", "one"],
        None,
    )
    .unwrap();
    command(
        root,
        &["config", "--add", "remote.origin.pushurl", "two"],
        None,
    )
    .unwrap();
    assert!(mutate(
        root,
        Mutation::SetRemote {
            name: "origin".into(),
            fetch_url: "https://example.com/new".into(),
            push_url: None
        }
    )
    .is_err());
    assert_eq!(
        remotes(root).unwrap()[0].fetch_url,
        "https://example.com/repo"
    );
}

#[test]
fn remote_mutations_accept_frontend_field_names() {
    let add = serde_json::json!({
        "kind": "addRemote", "name": "origin", "fetchUrl": "../remote.git", "pushUrl": null
    });
    assert!(matches!(
        serde_json::from_value::<Mutation>(add).unwrap(),
        Mutation::AddRemote { .. }
    ));
    let update = serde_json::json!({
        "kind": "setRemote", "name": "origin", "fetchUrl": "../other.git", "pushUrl": "../push.git"
    });
    assert!(matches!(
        serde_json::from_value::<Mutation>(update).unwrap(),
        Mutation::SetRemote { .. }
    ));
    let rename = serde_json::json!({
        "kind": "renameRemote", "oldName": "origin", "newName": "backup"
    });
    assert!(matches!(
        serde_json::from_value::<Mutation>(rename).unwrap(),
        Mutation::RenameRemote { .. }
    ));
}
