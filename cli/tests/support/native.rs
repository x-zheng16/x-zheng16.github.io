use super::Repo;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct Site {
    repo: Repo,
    root: PathBuf,
}

impl Site {
    fn new() -> Self {
        let repo = Repo::new();
        let root = repo.root.join(".git/site");
        repo.git(&[
            "clone",
            "--no-local",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_str()
                .unwrap(),
            root.to_str().unwrap(),
        ]);
        let site = Self { repo, root };
        fs::create_dir_all(site.repo.root.join(".git/tmp")).unwrap();
        site.git(&["branch", "-M", "master"]);
        site.git(&["config", "user.name", "Fixture"]);
        site.git(&["config", "user.email", "fixture@example.invalid"]);
        site.git(&[
            "remote",
            "set-url",
            "origin",
            site.repo.root.to_str().unwrap(),
        ]);
        site.run(site.command("bundle").arg("install"));
        assert_eq!(site.git(&["status", "--porcelain"]), "");
        site
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        let path = std::env::join_paths(
            [
                self.root.join(".git/helper/bin"),
                self.root.join(".git/node/node_modules/.bin"),
            ]
            .into_iter()
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let mut command = self.repo.command(program);
        command
            .current_dir(&self.root)
            .env("PATH", path)
            .env_remove("HOMEPAGE_BIN")
            .env_remove("PORT")
            .env("BUNDLE_FROZEN", "true")
            .env("TMPDIR", self.repo.root.join(".git/tmp"))
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .env("BU_HOME", self.repo.root.join(".git/bu"))
            .env("BU_PROFILE", "")
            .env("BU_AGENT", "homepage-e2e")
            .env("BU_EXIT_WITH", std::process::id().to_string())
            .env(
                "CC_CACHE_ROOT",
                std::env::var_os("CC_CACHE_ROOT")
                    .unwrap_or_else(|| home.join(".cache/cc").into_os_string()),
            )
            .env(
                "UV_CACHE_DIR",
                std::env::var_os("UV_CACHE_DIR")
                    .unwrap_or_else(|| home.join(".cache/uv").into_os_string()),
            )
            .env(
                "CARGO_HOME",
                std::env::var_os("CARGO_HOME")
                    .unwrap_or_else(|| home.join(".cargo").into_os_string()),
            )
            .env(
                "RUSTUP_HOME",
                std::env::var_os("RUSTUP_HOME")
                    .unwrap_or_else(|| home.join(".rustup").into_os_string()),
            );
        command
    }

    fn run(&self, command: &mut Command) -> String {
        let child = self
            .repo
            .launch(command.stdout(Stdio::piped()).stderr(Stdio::piped()));
        let pid = child.id();
        let output = child.wait_with_output().unwrap();
        self.repo.forget(pid);
        if command.get_program() != "git" || !output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(output.status.success(), "{command:?}: {}", output.status);
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_end()
            .to_owned()
    }

    fn git(&self, args: &[&str]) -> String {
        self.run(self.command("git").args(args))
    }
}

pub(super) fn stop_browser(root: &Path) {
    let home = root.join(".git/bu");
    if home.join("daemon.sock").exists() {
        let mut child = Command::new("bu")
            .arg("stop")
            .env("BU_HOME", home)
            .env("BU_PROFILE", "")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let _ = child.wait();
    }
}

fn preview_command(root: &Path) -> String {
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.split_whitespace().count() <= 400);
    readme
        .split("## Local preview\n")
        .nth(1)
        .expect("README local preview instructions")
        .split("```sh\n")
        .nth(1)
        .unwrap()
        .split("\n```")
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn native_readme_preview_from_clean_clone() {
    let site = Site::new();
    let command = preview_command(&site.root);
    assert_eq!(command, "bin/preview-local");
    let lock = fs::read(site.root.join("Gemfile.lock")).unwrap();
    drop(TcpListener::bind("127.0.0.1:4000").expect("preview test needs port 4000 free"));
    let mut server = site.repo.launch(
        site.command("sh")
            .args(["-c", &command])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()),
    );
    let stdout = server.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            println!("{line}");
            if line.contains("Server running") {
                let _ = tx.send(());
            }
        }
    });
    rx.recv_timeout(Duration::from_secs(300))
        .expect("Jekyll server startup within 300 seconds");
    let listeners =
        site.run(
            site.command("lsof")
                .args(["-nP", "-iTCP:4000", "-sTCP:LISTEN", "-Fn"]),
        );
    let addresses: Vec<_> = listeners
        .lines()
        .filter(|line| line.starts_with('n'))
        .collect();
    assert_eq!(addresses, ["n127.0.0.1:4000"]);
    let bu = site.repo.root.join(".git/bu");
    fs::create_dir_all(&bu).unwrap();
    fs::write(
        bu.join("config.toml"),
        "pipe = true\nheadless = true\nidentity = \"\"\nsites = []\nidle_quit = 0\n",
    )
    .unwrap();
    site.run(site.command("bu").arg("start"));
    site.run(site.command("bu").args([
        "run",
        r#"page = await tabs.open('about:blank');
await page.cdp('Network.enable');
const pages = [];
for (const [path, selector, expected] of [['/', 'main .hero h1', 'Xiang Zheng'], ['/publications/', 'main h1.ptitle', 'Publications']]) {
  const url = 'http://127.0.0.1:4000' + path;
  const received = browser.waitFor('Network.responseReceived', {sessionId: page.session, predicate: event => event.type === 'Document' && event.response.url === url});
  const [navigation, network] = await Promise.all([page.goto(url), received]);
  const heading = await page.evaluate(selector => document.querySelector(selector)?.firstChild?.textContent.trim(), selector);
  const result = {url: navigation.url, status: network.response.status, heading};
  if (result.url !== url || result.status !== 200 || heading !== expected) throw Error(JSON.stringify(result));
  pages.push(result);
}
const css = await page.evaluate(async () => { const r = await fetch('/assets/css/main.css'); return {status: r.status, type: r.headers.get('content-type'), length: (await r.text()).length}; });
if (css.status !== 200 || !css.type.includes('text/css') || css.length < 100) throw Error(JSON.stringify(css));
({pages, css})"#,
    ]));
    site.run(site.command("bu").arg("stop"));
    assert_eq!(lock, fs::read(site.root.join("Gemfile.lock")).unwrap());
    assert_eq!(site.git(&["status", "--porcelain"]), "");
    unsafe {
        libc::kill(-(server.id() as i32), libc::SIGINT);
    }
    server.wait().unwrap();
    site.repo.forget(server.id());
    reader.join().unwrap();
    drop(TcpListener::bind("127.0.0.1:4000").expect("preview listener stopped"));
}

#[test]
fn native_deploy_from_clean_clone() {
    let site = Site::new();
    site.run(
        site.command(env!("CARGO"))
            .args([
                "install", "--path", "cli", "--locked", "--bin", "homepage", "--root",
            ])
            .arg(site.root.join(".git/helper"))
            .arg("--target-dir")
            .arg(site.root.join(".git/target")),
    );
    let node = site.root.join(".git/node");
    fs::create_dir_all(&node).unwrap();
    site.run(
        site.command("pnpm")
            .arg("--dir")
            .arg(node)
            .args(["add", "purgecss"]),
    );
    let before = site.git(&["rev-parse", "HEAD"]);
    let mut child = site.repo.launch(
        site.command("uv")
            .args([
                "run",
                "--python",
                "3.13",
                "--no-project",
                "--with-requirements",
                "requirements.txt",
                "--",
                "bin/deploy",
                "--no-push",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    );
    let pid = child.id();
    child.stdin.take().unwrap().write_all(b"yes\n").unwrap();
    let output = child.wait_with_output().unwrap();
    site.repo.forget(pid);
    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
    assert_eq!(site.git(&["branch", "--show-current"]), "master");
    assert_eq!(site.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(site.git(&["rev-parse", "gh-pages^"]), before);
    assert_eq!(site.git(&["status", "--porcelain"]), "");
    assert!(
        site.git(&["show", "gh-pages:index.html"])
            .contains("Xiang Zheng")
    );
    assert!(site.git(&["show", "gh-pages:assets/css/main.css"]).len() > 100);
    assert_eq!(site.git(&["show", "gh-pages:.nojekyll"]), "");
    let paths = site.git(&["ls-tree", "-r", "--name-only", "gh-pages"]);
    assert!(!paths.lines().any(|path| path.starts_with("cli/")));
    assert!(site.repo.git(&["branch", "--list", "gh-pages"]).is_empty());
}
