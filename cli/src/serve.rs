use crate::{Error, Result, error};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

enum Event {
    Signal(i32),
    Server(u64, io::Result<()>),
    Watch(io::Result<()>),
}

struct Process {
    child: Option<Child>,
    waiter: Option<JoinHandle<()>>,
}

impl Process {
    fn spawn(command: &mut Command, tx: Sender<Event>, server: Option<u64>) -> Result<Self> {
        let child = command.process_group(0).spawn()?;
        let pid = child.id();
        let waiter = thread::spawn(move || {
            let status = loop {
                let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::uninit();
                let result = unsafe {
                    libc::waitid(
                        libc::P_PID,
                        pid,
                        info.as_mut_ptr(),
                        libc::WEXITED | libc::WNOWAIT,
                    )
                };
                if result == 0 {
                    break Ok(());
                }
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    break Err(error);
                }
            };
            let event = match server {
                Some(generation) => Event::Server(generation, status),
                None => Event::Watch(status),
            };
            let _ = tx.send(event);
        });
        Ok(Self {
            child: Some(child),
            waiter: Some(waiter),
        })
    }

    fn finish(&mut self) -> io::Result<ExitStatus> {
        let mut child = self.child.take().expect("child still owned");
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        if let Some(waiter) = self.waiter.take() {
            let _ = waiter.join();
        }
        let status = child.wait()?;
        loop {
            let result = unsafe { libc::waitpid(-(child.id() as i32), std::ptr::null_mut(), 0) };
            if result > 0 {
                continue;
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ECHILD) {
                break;
            }
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
        Ok(status)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.finish();
        }
    }
}

struct SignalThread {
    handle: signal_hook::iterator::Handle,
    thread: Option<JoinHandle<()>>,
}

impl SignalThread {
    fn new(tx: Sender<Event>) -> Result<Self> {
        let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP])?;
        let handle = signals.handle();
        let thread = thread::spawn(move || {
            for signal in signals.forever() {
                if tx.send(Event::Signal(signal)).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            handle,
            thread: Some(thread),
        })
    }
}

impl Drop for SignalThread {
    fn drop(&mut self) {
        self.handle.close();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn git() -> io::Result<Command> {
    let mut command = Command::new("git");
    command.arg("-c").arg(format!(
        "safe.directory={}",
        std::env::current_dir()?.display()
    ));
    Ok(command)
}

fn manage_lockfile() -> Result<()> {
    if !std::path::Path::new("Gemfile.lock").is_file() {
        return Ok(());
    }
    let tracked = match git()?
        .args(["ls-files", "--error-unmatch", "Gemfile.lock"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.success(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if tracked {
        println!("Gemfile.lock is tracked by git, keeping it intact");
        let _ = git()?
            .args(["restore", "Gemfile.lock"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else {
        println!("Gemfile.lock is not tracked by git, removing it");
        fs::remove_file("Gemfile.lock")?;
    }
    Ok(())
}

fn watch(tx: &Sender<Event>) -> Result<Process> {
    Process::spawn(
        Command::new("inotifywait").args(["-q", "-e", "modify,move,create,delete", "_config.yml"]),
        tx.clone(),
        None,
    )
}

fn start(tx: &Sender<Event>, generation: u64) -> Result<Process> {
    manage_lockfile()?;
    Process::spawn(
        Command::new("bundle").args([
            "exec",
            "jekyll",
            "serve",
            "--watch",
            "--port=8080",
            "--host=0.0.0.0",
            "--livereload",
            "--verbose",
            "--trace",
            "--force_polling",
        ]),
        tx.clone(),
        Some(generation),
    )
}

pub fn serve() -> Result<()> {
    #[cfg(target_os = "linux")]
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error().into());
    }
    let (tx, rx) = mpsc::channel();
    let _signals = SignalThread::new(tx.clone())?;
    let mut watcher = watch(&tx)?;
    let mut generation = 0;
    let mut server = start(&tx, generation)?;
    while let Ok(event) = rx.recv() {
        match event {
            Event::Signal(signal) => {
                return Err(Error {
                    message: String::new(),
                    code: (128 + signal) as u8,
                });
            }
            Event::Server(id, status) if id == generation => {
                status?;
                return Err(error(format!("Jekyll stopped: {}", server.finish()?)));
            }
            Event::Server(_, _) => {}
            Event::Watch(status) => {
                status?;
                let status = watcher.finish()?;
                if !status.success() {
                    return Err(error(format!("inotifywait stopped: {status}")));
                }
                println!("Change detected to _config.yml, restarting Jekyll");
                drop(watcher);
                watcher = watch(&tx)?;
                drop(server);
                generation += 1;
                server = start(&tx, generation)?;
            }
        }
    }
    Err(error("Entrypoint event channel closed."))
}
