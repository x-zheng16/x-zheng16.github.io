use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<_> = env::args().collect();
    let program = Path::new(&args[0]).file_name().unwrap().to_str().unwrap();
    match program {
        "bundle" => {
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
        "purgecss" => {
            assert_eq!(&args[1..], ["-c", "purgecss.config.js"]);
            if env::var("FIXTURE_FAIL").is_ok_and(|value| value == "purgecss") {
                process::exit(8);
            }
            fs::write("_site/assets/site.css", "purged").unwrap();
        }
        _ => panic!("unknown fixture command: {args:?}"),
    }
}
