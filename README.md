# srv

A tiny static web server. Serves the current folder over HTTP — single binary, standard library only, no dependencies.

```
srv — serve the current folder over HTTP

usage: srv [port] [--host name] [--public]

  port       port to listen on (default 8000)
  --host     name to show in the URL, e.g. home.lo (default localhost)
  --public   listen on all interfaces (default: localhost only)
  help       show this help
```

## behaviour

- Binds to 127.0.0.1 unless `--public` is given
- `--host home.lo` only changes the name shown in the URL; it must resolve to this machine, e.g. add `127.0.0.1  home.lo` to `/etc/hosts` (srv warns if it doesn't)
- GET and HEAD only; everything else returns 405
- Hides dotfiles, rejects `..`, and refuses symlinks that resolve outside the served folder
- Serves `index.html` for folders, and `/page` falls back to `page.html`
- Caps header size and concurrent connections (64), with 10 s I/O timeouts

## install

```bash
cargo install --path . --locked
```
