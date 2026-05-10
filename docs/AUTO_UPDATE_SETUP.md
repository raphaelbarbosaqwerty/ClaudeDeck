# Auto-update setup

This is a one-time setup that turns on the in-app auto-updater. After it's
done, every release tagged from `main` gets automatically delivered to
existing installs as a "Update available" toast in-app.

## How the updater works

```
┌──────────────────────────┐                  ┌──────────────────────────┐
│ Your local ClaudeDeck    │   1. fetch       │ GitHub release page      │
│   (running, v0.1.x)      │   ─────────────► │   …/latest/download/     │
│                          │                  │   latest.json            │
│                          │   2. compare ver │                          │
│  if remote > local:      │   3. download    │   uses *.app.tar.gz +    │
│    show toast            │   ─────────────► │   *.tar.gz.sig           │
│  on accept:              │   4. verify sig  │                          │
│    download + verify     │      against     │                          │
│    swap binary           │      pubkey      │                          │
│    relaunch              │                  │                          │
└──────────────────────────┘                  └──────────────────────────┘
```

The signature check uses **minisign** keys (Tauri's default). Updates fail
to install if the signature doesn't match the public key embedded in the
running binary, which prevents anyone with write access to the GitHub repo
(but without your private key) from pushing a malicious update.

## One-time setup

### 1. Generate the keypair

On your local machine, run:

```bash
cd /path/to/ClaudeDeck
pnpm tauri signer generate -- -w ~/.tauri/claudedeck.key
```

You'll be prompted for a passphrase — pick one and **store it in a
password manager**. You'll need it again when adding GitHub secrets.

The command writes two files:

- `~/.tauri/claudedeck.key` — the **private** key (DO NOT commit, DO NOT
  share). This signs new releases.
- `~/.tauri/claudedeck.key.pub` — the **public** key. Embedded in the app
  to verify signatures. Safe to commit.

### 2. Embed the public key in `tauri.conf.json`

```bash
cat ~/.tauri/claudedeck.key.pub
```

Copy the output and paste it as the `pubkey` value in
`src-tauri/tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "endpoints": [
      "https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/latest/download/latest.json"
    ],
    "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDE0...="
  }
}
```

(The pubkey is a long base64-ish string — paste the whole thing on one
line.)

### 3. Add the private key + password as GitHub repo secrets

Go to **Settings → Secrets and variables → Actions** on the repo and add:

| Secret name | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `~/.tauri/claudedeck.key` (open the file, copy the whole thing including the comment lines at the top) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The passphrase you chose in step 1 |

These are already wired into `.github/workflows/release.yml` as env vars
on the build step, so as soon as the secrets exist, signed updates start
shipping.

### 4. Cut the next release

Run a normal version bump and tag push:

```bash
pnpm bump 0.2.0
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: release v0.2.0 with auto-update support"
git tag v0.2.0
git push origin main v0.2.0
```

The release workflow now produces, in addition to the regular installers:

- `*.app.tar.gz` (macOS update bundle)
- `*.app.tar.gz.sig` (signature)
- `*.AppImage.tar.gz` + `.sig` (Linux)
- `*-setup.nsis.zip` + `.sig` (Windows)
- `latest.json` (manifest the updater fetches)

Once the release is **published** (not just draft), existing installs of
v0.1.x will see the update toast on next launch.

## What if I lose the private key?

Then signatures don't verify and existing installs can't auto-update —
they'll keep working but stop receiving updates silently. Recovery
options:

- **Generate a new keypair**, embed the new pubkey in `tauri.conf.json`,
  push a new release. Users on old versions still can't auto-update —
  they need to manually download the new binary one time.
- **Or**: ship a release that bumps the major version and openly
  documents the keypair rotation. Everyone re-installs once.

Treat the private key like an SSH key. Keep an offline backup.

## Caveats

- **Updates only work for builds produced by the same signing key.** The
  app currently running has v0.1.x's expected pubkey embedded — but
  v0.1.x didn't have an updater plugin at all, so existing v0.1.x users
  must download the first updater-enabled release (v0.2.0) manually. From
  v0.2.0 onward, auto-update kicks in.
- **macOS without code signing** still requires `xattr -cr` on the
  manually-downloaded v0.2.0. After that, auto-update writes the new
  bundle in-place and the quarantine attribute doesn't get re-applied.
- **Linux AppImage** auto-update works only when the `.AppImage` is
  marked executable. Users who run via package managers should disable
  the updater by removing the plugin block from their build.
