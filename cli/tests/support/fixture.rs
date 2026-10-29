use std::env;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{self, Command};

fn event(message: &str) -> UnixStream {
    let mut stream = UnixStream::connect(env::var_os("FIXTURE_SOCKET").unwrap()).unwrap();
    writeln!(stream, "{message}").unwrap();
    stream
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let program = Path::new(&args[0]).file_name().unwrap().to_str().unwrap();
    if args.get(1).is_some_and(|arg| arg == "worker") {
        eprintln!("fixture-worker-pid {}", process::id());
        let mut stream = event(&format!("worker {}", process::id()));
        let _ = stream.read(&mut [0]);
        return;
    }
    match program {
        "bundle" if args.iter().any(|arg| arg == "build") => {
            assert_eq!(&args[1..], ["exec", "jekyll", "build"]);
            assert_eq!(env::var("JEKYLL_ENV").unwrap(), "production");
            if env::var("FIXTURE_FAIL").is_ok_and(|value| value == "build") {
                process::exit(7);
            }
            fs::create_dir_all("_site/assets").unwrap();
            fs::write("_site/index.html", "built page").unwrap();
            fs::write("_site/assets/site.css", "css").unwrap();
            fs::write("_site/.hidden", "hidden").unwrap();
            fs::write("_site/CNAME", "built.example.invalid").unwrap();
        }
        "bundle" => {
            assert_eq!(
                &args[1..],
                [
                    "exec",
                    "jekyll",
                    "serve",
                    "--watch",
                    "--port=8080",
                    "--host=0.0.0.0",
                    "--livereload",
                    "--verbose",
                    "--trace",
                    "--force_polling"
                ]
            );
            if env::var("FIXTURE_FAIL").is_ok_and(|value| value == "serve") {
                process::exit(9);
            }
            let mut child = Command::new(env::current_exe().unwrap())
                .arg("worker")
                .spawn()
                .unwrap();
            eprintln!("fixture-worker-launch {}", child.id());
            let mut stream = event(&format!("serve {} {}", process::id(), child.id()));
            let _ = stream.read(&mut [0]);
            let _ = child.kill();
            let _ = child.wait();
        }
        "purgecss" => {
            assert_eq!(&args[1..], ["-c", "purgecss.config.js"]);
            if env::var("FIXTURE_FAIL").is_ok_and(|value| value == "purgecss") {
                process::exit(8);
            }
            fs::write("_site/assets/site.css", "purged").unwrap();
        }
        "inotifywait" => {
            assert_eq!(
                &args[1..],
                ["-q", "-e", "modify,move,create,delete", "_config.yml"]
            );
            let mut stream = event(&format!("watch {}", process::id()));
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            process::exit(if byte[0] == 1 { 0 } else { 6 });
        }
        _ => panic!("unknown fixture command: {args:?}"),
    }
}
