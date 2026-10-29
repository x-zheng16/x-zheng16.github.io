# Xiang Zheng's Personal Page

A Jekyll website based on al-folio. See [INSTALL.md](INSTALL.md) for Ruby, Bundler
and site dependencies, then run `bundle install` from the repository root.

## Deployment helper

Install Rust 1.85 or newer with Cargo, then build the helper:

```sh
cargo install --path cli --locked --bin homepage
bin/deploy --help
```

Run `bin/deploy` from a clean repository root. It asks for confirmation before
replacing the deployment branch and force-pushing it to `origin`. Install
`purgecss` on `PATH`, for example with `pnpm add --global purgecss`.
Use `bin/deploy --no-push` to prepare a local deployment branch without pushing.
Set `HOMEPAGE_BIN` to use a separately installed helper binary.

## Container preview

Install Docker with Compose, then run:

```sh
docker compose up --build
```

Open <http://localhost:8080>. Outside Docker, `bin/entry_point.sh` needs the built
helper, Bundler and `inotifywait` on `PATH` (Linux, package `inotify-tools`).
Use Ctrl-C to stop the preview.

## Tests

Install Git and Rust, then run the deployment and entrypoint scenarios:

```sh
cargo test --manifest-path cli/Cargo.toml --locked --features e2e --test e2e -- --test-threads=1
```
