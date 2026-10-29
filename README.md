# Xiang Zheng's Personal Page

A Jekyll website based on al-folio.

## Install

Install Git, Ruby 3.3 with Bundler 2.5.7, ImageMagick and
[uv](https://docs.astral.sh/uv/getting-started/installation/), and put their
executables on `PATH`. On Windows, use WSL. See [INSTALL.md](INSTALL.md) for
platform setup. From the repository root, run:

```sh
gem install bundler -v 2.5.7
bundle install
uv python install 3.13
```

## Local preview

```sh
bin/preview-local
```

Open <http://127.0.0.1:4000>. Use Ctrl-C to stop. Set `PORT` to choose another
port. Restart the command after changing `_config.yml`.

## Deployment helper

Install Rust 1.85 or newer with Cargo, Node.js 22 or newer, and
[pnpm](https://pnpm.io/installation). Then run:

```sh
cargo install --path cli --locked --bin homepage
pnpm add --global purgecss
bin/deploy --help
```

Put Cargo's and pnpm's binary directories on `PATH`. Run `bin/deploy` from a
clean repository root in the Python environment:

```sh
uv run --python 3.13 --no-project --with-requirements requirements.txt -- bin/deploy --no-push
```

Omit `--no-push` to publish. The helper asks for confirmation before replacing
the deployment branch and force-pushing it to `origin`. Set `HOMEPAGE_BIN` to
use a separately installed helper. See `bin/deploy --help` for options.

## Tests

Install the dependencies above and [bu](https://github.com/x-zheng16/cc-plugin-tool-use-browser-use)
with Chrome, plus `lsof`. Run individual Cargo integration scenarios:

```sh
cargo test --manifest-path cli/Cargo.toml --locked --features e2e --test e2e native::native_readme_preview_from_clean_clone -- --exact --test-threads=1
cargo test --manifest-path cli/Cargo.toml --locked --features e2e --test e2e native::native_deploy_from_clean_clone -- --exact --test-threads=1
```

Use `deploy_` instead of either scenario name, without `--exact`, for the
deployment option and failure scenarios.
