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

## prebuilt binary (Raspberry Pi 5, Raspberry Pi OS Lite / Debian Trixie, 64-bit)

Pushing a `v*` tag builds a static `aarch64-unknown-linux-musl` binary and attaches it to a GitHub release. Download it from the latest release, then:

```bash
tar -xzf srv-v*-linux-arm64-rpi5.tar.gz
sudo install -m 755 srv /usr/local/bin/srv
```

Verify the download against `SHA256SUMS.txt` from the same release with `sha256sum -c --ignore-missing SHA256SUMS.txt`.
