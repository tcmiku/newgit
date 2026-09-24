use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: u64 = 8 * 1024 * 1024;
pub type GitResult<T> = Result<T, String>;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub original_path: Option<String>,
    pub index: char,
    pub worktree: char,
    pub conflict: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub root: String,
    pub name: String,
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
    pub merging: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub patch: String,
    pub binary: bool,
    pub truncated: bool,
    pub can_stage_hunks: bool,
}

#[derive(Serialize)]
pub struct Commit {
    pub oid: String,
    pub short: String,
    pub subject: String,
    pub author: String,
    pub date: String,
    pub refs: String,
    pub parents: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLog {
    pub output: String,
    pub has_more: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub name: String,
    pub fetch_url: String,
    pub push_url: Option<String>,
}

#[derive(Serialize)]
pub struct Branch {
    pub name: String,
    pub current: bool,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Mutation {
    Stage {
        paths: Vec<String>,
    },
    Unstage {
        paths: Vec<String>,
    },
    StageHunk {
        path: String,
        expected: String,
        hunk: usize,
    },
    Commit {
        message: String,
    },
    SwitchBranch {
        name: String,
        create: bool,
    },
    Remote {
        operation: String,
        remote: Option<String>,
    },
    AddRemote {
        name: String,
        #[serde(rename = "fetchUrl")]
        fetch_url: String,
        #[serde(rename = "pushUrl")]
        push_url: Option<String>,
    },
    SetRemote {
        name: String,
        #[serde(rename = "fetchUrl")]
        fetch_url: String,
        #[serde(rename = "pushUrl")]
        push_url: Option<String>,
    },
    RenameRemote {
        #[serde(rename = "oldName")]
        old_name: String,
        #[serde(rename = "newName")]
        new_name: String,
    },
    RemoveRemote {
        name: String,
    },
    PublishBranch {
        remote: String,
    },
}

// Args never pass through a shell. Git reads literal filenames, including brackets and colons.
pub fn command(root: &Path, args: &[&str], input: Option<&str>) -> GitResult<Vec<u8>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(root)
        .args([
            "--no-pager",
            "--literal-pathspecs",
            "-c",
            "core.quotepath=false",
            "-c",
            "color.ui=false",
        ])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法运行 Git，请确认已安装 Git 并添加到 PATH。\n{e}"))?;
    let stdout = child.stdout.take().ok_or("无法读取 Git 输出")?;
    let stderr = child.stderr.take().ok_or("无法读取 Git 错误")?;
    // Drain both pipes concurrently; a full stderr pipe must never deadlock stdout.
    fn drain(mut source: impl Read) -> Vec<u8> {
        let mut bytes = Vec::new();
        let _ = source
            .by_ref()
            .take(OUTPUT_LIMIT + 1)
            .read_to_end(&mut bytes);
        let _ = std::io::copy(&mut source, &mut std::io::sink());
        bytes
    }
    let out = thread::spawn(move || drain(stdout));
    let err = thread::spawn(move || drain(stderr));
    let writer = input.map(|input| {
        let data = input.as_bytes().to_vec();
        let mut stdin = child.stdin.take().unwrap();
        thread::spawn(move || stdin.write_all(&data))
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if started.elapsed() > Duration::from_secs(120) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Git 操作超过 120 秒，已停止等待。请检查网络、认证或 hooks 后刷新仓库；操作结果以仓库实际状态为准。".into());
            }
            None => thread::sleep(Duration::from_millis(8)),
        }
    };
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    let stdout = out.join().map_err(|_| "读取 Git 输出失败")?;
    let stderr = err.join().map_err(|_| "读取 Git 错误失败")?;
    if !status.success() {
        let message = String::from_utf8_lossy(&stderr).trim().to_string();
        return Err(if message.is_empty() {
            String::from_utf8_lossy(&stdout).trim().to_string()
        } else {
            message
        });
    }
    if stdout.len() as u64 > OUTPUT_LIMIT {
        return Err("输出超过 8 MB，请缩小文件或仓库查询范围。".into());
    }
    Ok(stdout)
}

fn text(root: &Path, args: &[&str]) -> GitResult<String> {
    String::from_utf8(command(root, args, None)?)
        .map_err(|_| "该内容不是 UTF-8 文本，暂不支持显示。".into())
}

pub fn open(path: &str) -> GitResult<PathBuf> {
    let path = Path::new(path)
        .canonicalize()
        .map_err(|e| format!("无法打开文件夹：{e}"))?;
    if !path.is_dir() {
        return Err("请选择 Git 仓库文件夹。".into());
    }
    let root = text(&path, &["rev-parse", "--show-toplevel"])?;
    Path::new(root.trim())
        .canonicalize()
        .map_err(|e| e.to_string())
}

pub fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

pub fn git_dir(root: &Path) -> GitResult<PathBuf> {
    Ok(PathBuf::from(
        text(root, &["rev-parse", "--absolute-git-dir"])?.trim(),
    ))
}

pub fn parse_status(bytes: &[u8]) -> GitResult<Vec<FileChange>> {
    let mut entries = bytes.split(|b| *b == 0).filter(|entry| !entry.is_empty());
    let mut files = Vec::new();
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            return Err("Git 状态格式无效。".into());
        }
        let index = entry[0] as char;
        let worktree = entry[1] as char;
        let path = std::str::from_utf8(&entry[3..])
            .map_err(|_| "仓库包含非 UTF-8 文件名，暂不支持。")?
            .to_string();
        let original_path = if matches!(index, 'R' | 'C') || matches!(worktree, 'R' | 'C') {
            Some(
                std::str::from_utf8(entries.next().ok_or("缺少重命名前路径")?)
                    .map_err(|_| "文件名不是 UTF-8")?
                    .to_string(),
            )
        } else {
            None
        };
        let conflict =
            index == 'U' || worktree == 'U' || matches!((index, worktree), ('A', 'A') | ('D', 'D'));
        files.push(FileChange {
            path,
            original_path,
            index,
            worktree,
            conflict,
        });
    }
    Ok(files)
}

pub fn files(root: &Path) -> GitResult<Vec<FileChange>> {
    parse_status(&command(
        root,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
        None,
    )?)
}

pub fn snapshot(root: &Path) -> GitResult<Snapshot> {
    let files = files(root)?;
    let branch = text(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]).or_else(|_| {
        text(root, &["rev-parse", "--short", "HEAD"]).map(|s| format!("{} (detached)", s.trim()))
    })?;
    let upstream = text(
        root,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )
    .ok()
    .map(|s| s.trim().to_string());
    let (mut ahead, mut behind) = (0, 0);
    if upstream.is_some() {
        if let Ok(count) = text(
            root,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        ) {
            let values: Vec<&str> = count.split_whitespace().collect();
            ahead = values.first().and_then(|v| v.parse().ok()).unwrap_or(0);
            behind = values.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }
    let merging = git_dir(root)
        .map(|dir| {
            dir.join("MERGE_HEAD").exists()
                || dir.join("rebase-merge").exists()
                || dir.join("rebase-apply").exists()
        })
        .unwrap_or(false);
    Ok(Snapshot {
        root: display_path(root),
        name: root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        branch: branch.trim().into(),
        upstream,
        ahead,
        behind,
        files,
        merging,
    })
}

fn validate_path(path: &str) -> GitResult<()> {
    if path.is_empty()
        || path.contains('\0')
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("无效的仓库内相对路径。".into());
    }
    Ok(())
}

fn patch_for(root: &Path, path: &str, staged: bool) -> GitResult<String> {
    let mut args = vec![
        "--no-optional-locks",
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--unified=3",
    ];
    if staged {
        args.push("--cached");
    }
    args.extend(["--", path]);
    text(root, &args)
}

pub fn diff(root: &Path, path: &str, staged: bool) -> GitResult<Diff> {
    validate_path(path)?;
    let status = files(root)?;
    let file = status
        .iter()
        .find(|f| f.path == path)
        .ok_or("文件状态已变化，请刷新后重试。")?;
    if file.index == '?' && !staged {
        let full = root.join(path);
        let meta = std::fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Ok(Diff {
                patch: format!(
                    "符号链接 → {}",
                    std::fs::read_link(full)
                        .map_err(|e| e.to_string())?
                        .display()
                ),
                binary: true,
                truncated: false,
                can_stage_hunks: false,
            });
        }
        if meta.len() > 1024 * 1024 {
            return Ok(Diff {
                patch: String::new(),
                binary: false,
                truncated: true,
                can_stage_hunks: false,
            });
        }
        let bytes = std::fs::read(full).map_err(|e| e.to_string())?;
        let content = String::from_utf8(bytes.clone());
        if bytes.contains(&0) || content.is_err() {
            return Ok(Diff {
                patch: String::new(),
                binary: true,
                truncated: false,
                can_stage_hunks: false,
            });
        }
        let content = content.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let mut patch = format!(
            "--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n",
            lines.len()
        );
        for line in lines {
            patch.push('+');
            patch.push_str(line);
            patch.push('\n');
        }
        return Ok(Diff {
            patch,
            binary: false,
            truncated: false,
            can_stage_hunks: false,
        });
    }
    let patch = patch_for(root, path, staged)?;
    let binary = patch.contains("Binary files ") || patch.contains("GIT binary patch");
    let truncated = patch.len() > 1024 * 1024;
    let can_stage_hunks = !staged
        && !binary
        && !truncated
        && !file.conflict
        && file.worktree == 'M'
        && !patch.contains("old mode ")
        && !patch.contains("new file mode ");
    Ok(Diff {
        patch: if truncated { String::new() } else { patch },
        binary,
        truncated,
        can_stage_hunks,
    })
}

fn selected_paths(
    root: &Path,
    selected: &[String],
    include_original: bool,
) -> GitResult<Vec<String>> {
    if selected.is_empty() {
        return Err("请先选择文件。".into());
    }
    let status = files(root)?;
    let mut paths = Vec::new();
    for path in selected {
        validate_path(path)?;
        let file = status
            .iter()
            .find(|f| &f.path == path)
            .ok_or("文件状态已变化，请刷新后重试。")?;
        paths.push(path.clone());
        if include_original {
            if let Some(original) = &file.original_path {
                paths.push(original.clone());
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn remote_name(root: &Path, name: &str) -> GitResult<()> {
    if name.is_empty() || name.starts_with('-') || name.contains('\0') {
        return Err("远程名称无效。".into());
    }
    let reference = format!("refs/remotes/{name}/branch");
    command(root, &["check-ref-format", &reference], None)
        .map(|_| ())
        .map_err(|_| "远程名称无效。".into())
}

fn remote_url(url: &str) -> GitResult<()> {
    if url.trim().is_empty() || url.len() > 4096 || url.contains(['\0', '\r', '\n']) {
        return Err("请填写有效的远程地址。".into());
    }
    Ok(())
}

fn explicit_push_urls(root: &Path, name: &str) -> GitResult<Vec<String>> {
    remote_name(root, name)?;
    let key = format!("remote.{name}.pushurl");
    Ok(text(root, &["config", "--get-all", &key])
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect())
}

pub fn remotes(root: &Path) -> GitResult<Vec<RemoteInfo>> {
    let names = text(root, &["remote"])?;
    names
        .lines()
        .map(|name| {
            remote_name(root, name)?;
            let key = format!("remote.{name}.url");
            let fetch_url = text(root, &["config", "--get", &key])?
                .trim_end()
                .to_string();
            let push_url = explicit_push_urls(root, name)?.into_iter().next();
            Ok(RemoteInfo {
                name: name.into(),
                fetch_url,
                push_url,
            })
        })
        .collect()
}

fn existing_remote(root: &Path, name: &str) -> GitResult<()> {
    remote_name(root, name)?;
    if !remotes(root)?.iter().any(|remote| remote.name == name) {
        return Err(format!("远程 {name} 已不存在，请刷新后重试。"));
    }
    Ok(())
}

pub fn mutate(root: &Path, action: Mutation) -> GitResult<String> {
    match action {
        Mutation::Stage { paths } => {
            let paths = selected_paths(root, &paths, false)?;
            let mut args = vec!["add", "--"];
            args.extend(paths.iter().map(String::as_str));
            command(root, &args, None)?;
            Ok("已暂存更改".into())
        }
        Mutation::Unstage { paths } => {
            let paths = selected_paths(root, &paths, true)?;
            let has_head = command(root, &["rev-parse", "--verify", "HEAD"], None).is_ok();
            let mut args = if has_head {
                vec!["reset", "-q", "HEAD", "--"]
            } else {
                vec!["rm", "--cached", "-f", "--"]
            };
            args.extend(paths.iter().map(String::as_str));
            command(root, &args, None)?;
            Ok("已取消暂存，工作区内容保持不变".into())
        }
        Mutation::StageHunk {
            path,
            expected,
            hunk,
        } => {
            let current = diff(root, &path, false)?;
            if !current.can_stage_hunks || current.patch != expected {
                return Err("文件已发生变化，请刷新差异后重新暂存。".into());
            }
            let first = expected.find("\n@@ ").ok_or("没有可暂存的代码块")? + 1;
            let mut starts: Vec<usize> = expected
                .match_indices("\n@@ ")
                .map(|(i, _)| i + 1)
                .collect();
            starts.push(expected.len());
            if hunk + 1 >= starts.len() {
                return Err("代码块不存在。".into());
            }
            let patch = format!(
                "{}{}",
                &expected[..first],
                &expected[starts[hunk]..starts[hunk + 1]]
            );
            command(
                root,
                &["apply", "--cached", "--check", "--whitespace=nowarn", "-"],
                Some(&patch),
            )?;
            command(
                root,
                &["apply", "--cached", "--whitespace=nowarn", "-"],
                Some(&patch),
            )?;
            Ok("已暂存代码块".into())
        }
        Mutation::Commit { message } => {
            if message.trim().is_empty() {
                return Err("请填写提交说明。".into());
            }
            if files(root)?.iter().any(|f| f.conflict) {
                return Err("请先解决冲突并暂存结果，再提交。".into());
            }
            if command(root, &["diff", "--cached", "--quiet", "--exit-code"], None).is_ok() {
                return Err("暂存区为空，请先暂存更改。".into());
            }
            command(root, &["commit", "--file=-"], Some(message.trim()))?;
            Ok("提交成功".into())
        }
        Mutation::SwitchBranch { name, create } => {
            if name.starts_with('-') || name.is_empty() {
                return Err("无效的分支名。".into());
            }
            command(root, &["check-ref-format", "--branch", &name], None)?;
            let args = if create {
                vec!["switch", "-c", &name]
            } else {
                vec!["switch", &name]
            };
            command(root, &args, None)?;
            Ok(format!("已切换到 {name}"))
        }
        Mutation::Remote { operation, remote } => {
            if let Some(ref name) = remote {
                existing_remote(root, name)?;
            }
            let args = match operation.as_str() {
                "fetch" => {
                    if let Some(name) = remote.as_deref() {
                        vec!["fetch", "--", name]
                    } else {
                        vec!["fetch", "--all"]
                    }
                }
                "pull" if remote.is_none() => vec!["pull", "--ff-only"],
                "push" if remote.is_none() => vec!["push"],
                _ => return Err("不支持的远程操作。".into()),
            };
            command(root, &args, None)?;
            Ok(format!("{operation} 完成"))
        }
        Mutation::AddRemote {
            name,
            fetch_url,
            push_url,
        } => {
            remote_name(root, &name)?;
            remote_url(&fetch_url)?;
            if let Some(ref push) = push_url {
                remote_url(push)?;
            }
            command(root, &["remote", "add", "--", &name, &fetch_url], None)?;
            if let Some(push) = push_url {
                command(
                    root,
                    &["remote", "set-url", "--push", "--", &name, &push],
                    None,
                )?;
            }
            Ok(format!("已添加远程 {name}"))
        }
        Mutation::SetRemote {
            name,
            fetch_url,
            push_url,
        } => {
            existing_remote(root, &name)?;
            remote_url(&fetch_url)?;
            if let Some(ref push) = push_url {
                remote_url(push)?;
            }
            let old_push = explicit_push_urls(root, &name)?;
            if old_push.len() > 1 {
                return Err("该远程配置了多个推送地址，请用 Git 命令行管理。".into());
            }
            command(root, &["remote", "set-url", "--", &name, &fetch_url], None)?;
            if let Some(push) = push_url {
                command(
                    root,
                    &["remote", "set-url", "--push", "--", &name, &push],
                    None,
                )?;
            } else if !old_push.is_empty() {
                let key = format!("remote.{name}.pushurl");
                command(root, &["config", "--unset-all", &key], None)?;
            }
            Ok(format!("已更新远程 {name}"))
        }
        Mutation::RenameRemote { old_name, new_name } => {
            existing_remote(root, &old_name)?;
            remote_name(root, &new_name)?;
            command(
                root,
                &["remote", "rename", "--", &old_name, &new_name],
                None,
            )?;
            Ok(format!("已将远程 {old_name} 重命名为 {new_name}"))
        }
        Mutation::RemoveRemote { name } => {
            existing_remote(root, &name)?;
            command(root, &["remote", "remove", "--", &name], None)?;
            Ok(format!("已移除远程 {name}"))
        }
        Mutation::PublishBranch { remote } => {
            existing_remote(root, &remote)?;
            command(root, &["rev-parse", "--verify", "HEAD"], None)
                .map_err(|_| "请先创建一次提交，再发布分支。")?;
            let branch = text(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
                .map_err(|_| "当前处于 detached HEAD，无法发布分支。")?;
            let branch = branch.trim();
            command(
                root,
                &["push", "--set-upstream", "--", &remote, branch],
                None,
            )?;
            Ok(format!("已将 {branch} 发布到 {remote} 并设置跟踪分支"))
        }
    }
}

pub fn history(root: &Path, offset: usize, all: bool) -> GitResult<Vec<Commit>> {
    if !all && command(root, &["rev-parse", "--verify", "HEAD"], None).is_err() {
        return Ok(Vec::new());
    }
    let skip = format!("--skip={}", offset.min(1_000_000));
    let mut args = vec![
        "log",
        "--topo-order",
        "-z",
        "--max-count=60",
        &skip,
        "--format=%H%x00%h%x00%s%x00%an%x00%aI%x00%D%x00%P",
    ];
    if all {
        args.push("--all");
        if command(root, &["rev-parse", "--verify", "HEAD"], None).is_ok() {
            args.push("HEAD");
        }
    }
    let raw = text(root, &args)?;
    let fields: Vec<&str> = raw.split('\0').collect();
    Ok(fields
        .chunks_exact(7)
        .map(|f| Commit {
            oid: f[0].into(),
            short: f[1].into(),
            subject: f[2].into(),
            author: f[3].into(),
            date: f[4].into(),
            refs: f[5].into(),
            parents: f[6].split_whitespace().map(str::to_string).collect(),
        })
        .collect())
}

pub fn git_log(root: &Path, limit: usize, all: bool) -> GitResult<GitLog> {
    let has_head = command(root, &["rev-parse", "--verify", "HEAD"], None).is_ok();
    if !has_head && (!all || command(root, &["show-ref", "--quiet"], None).is_err()) {
        return Ok(GitLog {
            output: String::new(),
            has_more: false,
        });
    }

    let limit = limit.clamp(1, 5000);
    let count_limit = format!("--max-count={}", limit + 1);
    let mut count_args = vec!["rev-list", "--count", &count_limit];
    if all {
        count_args.push("--all");
        if has_head {
            count_args.push("HEAD");
        }
    } else {
        count_args.push("HEAD");
    }
    let count = text(root, &count_args)?
        .trim()
        .parse::<usize>()
        .map_err(|e| e.to_string())?;

    let max_count = format!("--max-count={limit}");
    let mut args = vec![
        "log",
        "--graph",
        "--topo-order",
        "--decorate=short",
        "--date=short",
        "--no-color",
        &max_count,
        "--pretty=format:%h %d %s  [%an · %ad]",
    ];
    if all {
        args.push("--all");
        if has_head {
            args.push("HEAD");
        }
    } else {
        args.push("HEAD");
    }
    Ok(GitLog {
        output: text(root, &args)?,
        has_more: count > limit,
    })
}

pub fn branches(root: &Path) -> GitResult<Vec<Branch>> {
    let raw = text(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(HEAD)%09%(refname:short)",
            "refs/heads/",
        ],
    )?;
    Ok(raw
        .lines()
        .filter_map(|line| {
            line.split_once('\t').map(|(head, name)| Branch {
                name: name.into(),
                current: head == "*",
            })
        })
        .collect())
}

pub fn commit_patch(root: &Path, oid: &str) -> GitResult<Diff> {
    if oid.len() != 40 && oid.len() != 64 || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("无效的提交 ID。".into());
    }
    let patch = text(
        root,
        &[
            "show",
            "--format=",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--stat",
            "--patch",
            "--diff-merges=first-parent",
            oid,
            "--",
        ],
    )?;
    let truncated = patch.len() > 1024 * 1024;
    Ok(Diff {
        patch: if truncated { String::new() } else { patch },
        binary: false,
        truncated,
        can_stage_hunks: false,
    })
}

#[cfg(test)]
mod tests;
