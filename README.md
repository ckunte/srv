# srv

A tiny static web server. Serves the current folder over HTTP — single binary, standard library only, no dependencies.

```
srv — serve the current folder over HTTP

usage: srv [port] [--public]

  port       port to listen on (default 8000)
  --public   listen on all interfaces (default: localhost only)
  help       show this help
```

## behaviour

- Binds to 127.0.0.1 unless `--public` is given
- GET and HEAD only; everything else returns 405
- Hides dotfiles, rejects `..`, and refuses symlinks that resolve outside the served folder
- Serves `index.html` for folders, and `/page` falls back to `page.html`
- Caps header size and concurrent connections (64), with 10 s I/O timeouts

## install

```bash
cargo install --path . --locked
```

## prebuilt binary (Linux arm64, e.g. Raspberry Pi 5)

CI builds a static `aarch64-unknown-linux-musl` binary. Grab `srv-linux-arm64.tar.gz` from the latest release (pushed `v*` tags) or from the workflow run's artifacts, then:

```bash
tar -xzf srv-linux-arm64.tar.gz
sudo install -m 755 srv /usr/local/bin/srv
```
