use super::{Repo, Running, closed_child, next, owned};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::Stdio;

#[test]
fn fixture_worker_pid_output_survives_socket_failures() {
    let repo = Repo::new();
    let missing = repo.root.join("missing.sock");
    let child = repo.launch(
        repo.command(env!("CARGO_BIN_EXE_entrypoint-fixture"))
            .arg("worker")
            .env("FIXTURE_SOCKET", &missing)
            .stderr(Stdio::piped()),
    );
    let pid = child.id();
    let output = child.wait_with_output().unwrap();
    repo.forget(pid);
    std::io::stderr().lock().write_all(&output.stderr).unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr.lines().next().unwrap(),
        format!("fixture-worker-pid {pid}")
    );

    let rx = repo.events();
    let worker = Running {
        child: repo.launch(
            repo.command(env!("CARGO_BIN_EXE_entrypoint-fixture"))
                .arg("worker")
                .env("FIXTURE_SOCKET", repo.root.join("events.sock")),
        ),
        root: repo.root.clone(),
    };
    let pid = worker.child.id();
    let (message, stream) = next(&rx);
    assert_eq!(message, format!("worker {pid}"));
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _worker = worker;
        UnixStream::connect(missing).unwrap();
    }));
    assert!(failure.is_err());
    closed_child((pid, stream));
    assert!(owned().lock().unwrap().get(&repo.root).unwrap().is_empty());
    let root = repo.root.clone();
    drop(repo);
    assert!(!root.exists());
}
