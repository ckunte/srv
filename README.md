# srv

A tiny static file server. Serves the current folder over HTTP — single binary, standard library only, no dependencies.

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

Extracted from [hkp](https://github.com/ckunte/hkp), where it was the `srv` command.
