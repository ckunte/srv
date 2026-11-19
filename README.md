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

## setup: srv on Raspberry Pi OS Lite

Serve a static site from a Raspberry Pi 5 on your LAN, started at boot by systemd. Steps assume Raspberry Pi OS Lite (64-bit, Debian Trixie) and a release already published.

**1. Install the binary**

```bash
VERSION=v0.1.0   # the release tag to install
cd /tmp
curl -LO https://github.com/ckunte/srv/releases/download/$VERSION/srv-$VERSION-linux-arm64-rpi5.tar.gz
curl -LO https://github.com/ckunte/srv/releases/download/$VERSION/SHA256SUMS.txt
sha256sum -c --ignore-missing SHA256SUMS.txt
tar -xzf srv-$VERSION-linux-arm64-rpi5.tar.gz
sudo install -m 755 srv /usr/local/bin/srv
srv help
```

**2. Create a service user and the site folder**

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin srv
sudo mkdir -p /var/www/site
sudo chown -R $USER:srv /var/www/site
sudo chmod -R g+rX /var/www/site
```

You own the files and can edit them; the `srv` user can only read them. Keep everything inside this folder, because `srv` refuses symlinks that point outside it.

**3. Create the systemd service**

```bash
sudo tee /etc/systemd/system/srv.service >/dev/null <<'UNIT'
[Unit]
Description=srv static site
After=network-online.target
Wants=network-online.target

[Service]
User=srv
Group=srv
WorkingDirectory=/var/www/site
ExecStart=/usr/local/bin/srv 80 --public
AmbientCapabilities=CAP_NET_BIND_SERVICE
Restart=on-failure
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
UNIT
```

`WorkingDirectory` is the folder that gets served. `--public` is needed for other devices on the LAN to connect. To use the default port 8000 instead of 80, change `80` to `8000` and drop the `AmbientCapabilities` line.

**4. Start it and enable it at boot**

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now srv
systemctl status srv
```

**5. Test**

```bash
curl -I http://localhost/2021/jf    # on the Pi; serves 2021/jf.html
```

From another device on the LAN, open `http://<pi-ip>/2021/jf` or `http://raspberrypi.local/2021/jf`.

**6. Deploy content** (from your main machine)

```bash
rsync -av --delete ./site/ pi@raspberrypi.local:/var/www/site/
```

No restart is needed; files are read from disk on each request.

**7. Upgrade srv**

Repeat step 1 with the new `VERSION`, then `sudo systemctl restart srv`.

Logs: `journalctl -u srv -n 50 --no-pager`.

**Notes**

- There is no HTTPS. Keep it on your LAN; don't expose the port to the internet.
- Give the Pi a fixed IP with a DHCP reservation on your router so the address doesn't change.
