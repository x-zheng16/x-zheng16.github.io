#[path = "support/native.rs"]
mod native;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::{fs::symlink, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
};

type Processes = Arc<Mutex<BTreeMap<PathBuf, Vec<u32>>>>;

static NEXT: AtomicU64 = AtomicU64::new(0);
static OWNED: OnceLock<Processes> = OnceLock::new();

fn owned() -> &'static Processes {
    OWNED.get_or_init(|| {
        let state = Arc::new(Mutex::new(BTreeMap::<PathBuf, Vec<u32>>::new()));
        let cleanup = state.clone();
        let mut signals =
            signal_hook::iterator::Signals::new([libc::SIGINT, libc::SIGTERM, libc::SIGHUP])
                .unwrap();
        std::thread::spawn(move || {
            if let Some(signal) = signals.forever().next() {
                for (path, pids) in cleanup.lock().unwrap().iter() {
                    native::stop_browser(path);
                    for pid in pids {
                        unsafe {
                            libc::kill(-(*pid as i32), libc::SIGTERM);
                            libc::waitpid(*pid as i32, std::ptr::null_mut(), 0);
                        }
                    }
                    let _ = fs::remove_dir_all(path);
                }
                std::process::exit(128 + signal);
            }
        });
        state
    })
}

struct Repo {
    root: PathBuf,
    bin: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("cc-scratch")
            .join(format!(
                "e2e-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        let mut state = owned().lock().unwrap();
        fs::create_dir_all(&root).unwrap();
        state.insert(root.clone(), Vec::new());
        drop(state);
        let repo = Self {
            bin: root.join(".git/commands"),
            root,
        };
        fs::create_dir_all(&repo.bin).unwrap();
        fs::create_dir_all(repo.root.join(".git/home")).unwrap();
        fs::write(repo.root.join(".gitignore"), "_site/\nGemfile.lock\n").unwrap();
        fs::write(repo.root.join("_config.yml"), "title: fixture\n").unwrap();
        fs::write(repo.root.join("source.txt"), "source\n").unwrap();
        fs::write(repo.root.join("CNAME"), "example.invalid\n").unwrap();
        for name in ["bundle", "purgecss"] {
            symlink(env!("CARGO_BIN_EXE_deploy-fixture"), repo.bin.join(name)).unwrap();
        }
        repo.git(&["init", "-b", "master"]);
        repo.git(&["config", "user.name", "Fixture"]);
        repo.git(&["config", "user.email", "fixture@example.invalid"]);
        repo.git(&["add", "."]);
        repo.git(&["commit", "-m", "Fixture source"]);
        repo
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        let path = std::env::join_paths(
            std::iter::once(self.bin.clone())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        command
            .current_dir(&self.root)
            .env("PATH", path)
            .env("HOME", self.root.join(".git/home"))
            .env("XDG_CONFIG_HOME", self.root.join(".git/home"))
            .env("GIT_CONFIG_GLOBAL", self.root.join(".git/home/gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOMEPAGE_BIN", env!("CARGO_BIN_EXE_homepage"));
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_")
                && name != "GIT_CONFIG_GLOBAL"
                && name != "GIT_CONFIG_NOSYSTEM"
            {
                command.env_remove(name);
            }
        }
        command
    }

    fn launch(&self, command: &mut Command) -> Child {
        let mut state = owned().lock().unwrap();
        let child = command.process_group(0).spawn().unwrap();
        state.get_mut(&self.root).unwrap().push(child.id());
        drop(state);
        child
    }

    fn forget(&self, pid: u32) {
        owned()
            .lock()
            .unwrap()
            .get_mut(&self.root)
            .unwrap()
            .retain(|owned| *owned != pid);
    }

    fn git(&self, args: &[&str]) -> String {
        let child = self.launch(
            self.command("git")
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        );
        let pid = child.id();
        let output = child.wait_with_output().unwrap();
        self.forget(pid);
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_end()
            .to_owned()
    }

    fn deploy(&self, args: &[&str], answer: &str, fail: &str) -> Output {
        let entry = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bin/deploy");
        let mut command = self.command(entry);
        command
            .args(args)
            .env("FIXTURE_FAIL", fail)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = self.launch(&mut command);
        let pid = child.id();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(answer.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        self.forget(pid);
        output
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        native::stop_browser(&self.root);
        if let Some(pids) = owned().lock().unwrap().remove(&self.root) {
            for pid in pids {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGTERM);
                    libc::waitpid(pid as i32, std::ptr::null_mut(), 0);
                }
            }
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn deploy_confirmation_and_options() {
    let repo = Repo::new();
    let before = repo.git(&["rev-parse", "HEAD"]);
    for answer in ["n\n", "\n", ""] {
        let output = repo.deploy(&["--no-push"], answer, "");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Aborting."));
        assert_eq!(repo.git(&["rev-parse", "HEAD"]), before);
    }
    assert!(repo.deploy(&["--help"], "", "").status.success());
    for args in [vec!["--wat"], vec!["-s"], vec!["-d"]] {
        assert!(!repo.deploy(&args, "", "").status.success());
    }
}

#[test]
fn deploy_rejects_dirty_and_untracked_files() {
    let repo = Repo::new();
    fs::write(repo.root.join("source.txt"), "dirty\n").unwrap();
    let output = repo.deploy(&["--no-push"], "yes\n", "");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("uncommitted"));
    repo.git(&["restore", "source.txt"]);
    fs::write(repo.root.join("untracked"), "new\n").unwrap();
    let output = repo.deploy(&["--no-push"], "y\n", "");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("untracked"));
    assert_eq!(repo.git(&["branch", "--show-current"]), "master");
}

#[test]
fn deploy_no_push_and_exact_branch_names() {
    let repo = Repo::new();
    repo.git(&["branch", "gh-pages-old"]);
    let output = repo.deploy(
        &[
            "-u",
            "--src",
            "preview",
            "--deploy",
            "pages",
            "--verbose",
            "--no-push",
        ],
        "YES\n",
        "",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repo.git(&["branch", "--show-current"]), "preview");
    assert!(
        repo.git(&["show", "pages:index.html"])
            .contains("built page")
    );
    assert_eq!(repo.git(&["show", "pages:assets/site.css"]), "purged");
    assert_eq!(repo.git(&["show", "pages:.nojekyll"]), "");
    assert_eq!(repo.git(&["show", "pages:CNAME"]), "built.example.invalid");
    assert_eq!(repo.git(&["show", "pages:.hidden"]), "hidden");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B", "pages"]),
        "Fixture source [ci skip]"
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
    repo.git(&["show-ref", "--verify", "refs/heads/gh-pages-old"]);
    let output = repo.deploy(&["-s", "preview", "-d", "pages", "--no-push"], "y\n", "");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn deploy_pushes_only_to_local_remote() {
    let repo = Repo::new();
    repo.git(&["init", "--bare", ".git/home/remote.git"]);
    repo.git(&["remote", "add", "origin", ".git/home/remote.git"]);
    let output = repo.deploy(&[], "yes\n", "");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        repo.git(&["rev-parse", "gh-pages"]),
        repo.git(&["--git-dir=.git/home/remote.git", "rev-parse", "gh-pages"])
    );
    assert_eq!(repo.git(&["branch", "--show-current"]), "master");
}

#[test]
fn deploy_stops_on_build_failures() {
    for failure in ["build", "purgecss"] {
        let repo = Repo::new();
        let output = repo.deploy(&["--no-push"], "y\n", failure);
        assert!(!output.status.success());
        assert_eq!(repo.git(&["branch", "--show-current"]), "gh-pages");
        assert!(repo.root.join("source.txt").is_file());
    }
}

#[test]
fn deploy_uses_existing_source_and_retains_push_failure_branch() {
    let repo = Repo::new();
    repo.git(&["checkout", "-b", "source"]);
    fs::write(repo.root.join("source.txt"), "other source\n").unwrap();
    repo.git(&["commit", "-am", "Other source"]);
    let source = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["checkout", "master"]);
    let output = repo.deploy(&["-s", "source", "--no-push"], "y\n", "");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repo.git(&["branch", "--show-current"]), "source");
    assert_eq!(repo.git(&["rev-parse", "gh-pages^"]), source);
    let output = repo.deploy(&["-s", "source"], "y\n", "");
    assert!(!output.status.success());
    assert_eq!(repo.git(&["branch", "--show-current"]), "gh-pages");
    assert!(repo.root.join("index.html").is_file());
}

#[test]
fn source_only_package_builds_and_runs_deploy() {
    let repo = Repo::new();
    let package = repo.root.join(".git/package");
    fs::create_dir_all(package.join("src")).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in ["Cargo.toml", "Cargo.lock", "src/main.rs", "src/deploy.rs"] {
        fs::copy(source.join(file), package.join(file)).unwrap();
    }
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let mut command = repo.command(env!("CARGO"));
    command
        .args([
            "install",
            "--locked",
            "--offline",
            "--bin",
            "homepage",
            "--path",
        ])
        .arg(&package)
        .arg("--root")
        .arg(package.join("install"))
        .arg("--target-dir")
        .arg(package.join("target"))
        .env(
            "CARGO_HOME",
            std::env::var_os("CARGO_HOME").unwrap_or_else(|| home.join(".cargo").into_os_string()),
        )
        .env(
            "RUSTUP_HOME",
            std::env::var_os("RUSTUP_HOME")
                .unwrap_or_else(|| home.join(".rustup").into_os_string()),
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut child = repo.launch(&mut command);
    let status = child.wait().unwrap();
    repo.forget(child.id());
    assert!(status.success());
    let entry = source.join("../bin/deploy");
    let child = repo.launch(
        repo.command(entry)
            .env_remove("HOMEPAGE_BIN")
            .env(
                "PATH",
                std::env::join_paths(
                    std::iter::once(package.join("install/bin"))
                        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
                )
                .unwrap(),
            )
            .arg("--help")
            .stdout(Stdio::piped()),
    );
    let pid = child.id();
    let output = child.wait_with_output().unwrap();
    repo.forget(pid);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("usage: deploy "));
}
