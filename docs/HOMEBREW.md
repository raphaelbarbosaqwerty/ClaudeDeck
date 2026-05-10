# Homebrew tap setup

A separate `homebrew-tap` repository lets users install ClaudeDeck with:

```bash
brew install raphaelbarbosaqwerty/tap/claudedeck
```

This is a one-time setup. After it's wired up, every published release
just needs the formula's version + sha256 to be bumped — which can also
be automated.

---

## 1. Create the tap repository

Create a new public repo on GitHub named exactly:

```
homebrew-tap
```

It must be `homebrew-tap` (the `homebrew-` prefix is mandatory; brew
strips it when resolving `user/tap` to `user/homebrew-tap`).

Initialize it with a basic README and clone locally.

## 2. Add the formula

Inside `homebrew-tap`, create `Formula/claudedeck.rb`:

```ruby
class Claudedeck < Formula
  desc "Multi-session Claude Code orchestrator with worktree isolation and agent dispatch"
  homepage "https://github.com/raphaelbarbosaqwerty/ClaudeDeck"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/download/v#{version}/claudedeck_#{version}_aarch64.dmg"
      sha256 "REPLACE_WITH_SHA256_OF_AARCH64_DMG"
    end
    on_intel do
      url "https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/download/v#{version}/claudedeck_#{version}_x64.dmg"
      sha256 "REPLACE_WITH_SHA256_OF_X64_DMG"
    end
  end

  def install
    # Mount the DMG, copy claudedeck.app into the Cask path, unmount.
    # `brew` already handles DMG mounting via the cask machinery for
    # `*.app` installs. Brew formulas typically use `bin.install`, but
    # for GUI macOS apps we point at the .app bundle in /Applications.
    prefix.install Dir["claudedeck.app"]
    bin.write_exec_script "#{prefix}/claudedeck.app/Contents/MacOS/claudedeck"
  end

  def caveats
    <<~EOS
      ClaudeDeck is not yet code-signed. After install, run:

        xattr -cr "#{prefix}/claudedeck.app"

      Or use the GUI: System Settings → Privacy & Security → Open Anyway.
    EOS
  end

  test do
    assert_predicate prefix/"claudedeck.app/Contents/MacOS/claudedeck", :executable?
  end
end
```

> ⚠ The `install` block above uses the basic formula approach. A more
> idiomatic alternative is to publish ClaudeDeck as a **Cask** instead of
> a Formula — Casks are designed specifically for `.dmg` / `.app`
> distributions. Cask example below.

### Alternative: Cask (recommended for GUI apps)

Casks live in `Casks/` rather than `Formula/`. Create
`Casks/claudedeck.rb`:

```ruby
cask "claudedeck" do
  version "0.2.0"
  sha256 arm:   "REPLACE_WITH_SHA256_OF_AARCH64_DMG",
         intel: "REPLACE_WITH_SHA256_OF_X64_DMG"

  url "https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/download/v#{version}/claudedeck_#{version}_#{arch}.dmg"
  name "ClaudeDeck"
  desc "Multi-session Claude Code orchestrator with worktree isolation and agent dispatch"
  homepage "https://github.com/raphaelbarbosaqwerty/ClaudeDeck"

  app "claudedeck.app"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-cr", "#{appdir}/claudedeck.app"]
  end

  zap trash: [
    "~/Library/Application Support/com.intuitive.claudedeck",
    "~/Library/Caches/com.intuitive.claudedeck",
    "~/Library/Preferences/com.intuitive.claudedeck.plist",
  ]
end
```

Users then install with:

```bash
brew install --cask raphaelbarbosaqwerty/tap/claudedeck
```

The `postflight` step strips quarantine automatically — no `xattr -cr`
needed on the user's side. The `zap` block defines what `brew uninstall
--zap` removes for full cleanup.

## 3. Compute the sha256 of each released DMG

After cutting a release, grab the sha256s:

```bash
curl -L -o /tmp/claudedeck-arm.dmg \
  https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/download/v0.2.0/claudedeck_0.2.0_aarch64.dmg
shasum -a 256 /tmp/claudedeck-arm.dmg

curl -L -o /tmp/claudedeck-intel.dmg \
  https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/download/v0.2.0/claudedeck_0.2.0_x64.dmg
shasum -a 256 /tmp/claudedeck-intel.dmg
```

Paste the hashes into the formula/cask.

## 4. Test locally before pushing

```bash
brew install --cask --HEAD raphaelbarbosaqwerty/homebrew-tap/Casks/claudedeck.rb
brew uninstall --cask claudedeck
```

If it installs and launches, push:

```bash
cd homebrew-tap
git add Casks/claudedeck.rb
git commit -m "Add ClaudeDeck v0.2.0 cask"
git push origin main
```

Now anyone can:

```bash
brew install --cask raphaelbarbosaqwerty/tap/claudedeck
```

## 5. Automate the bump (optional, but worth it)

Per release, the only thing that changes is `version` and the sha256 pair.
A GitHub Action in the `homebrew-tap` repo can listen to ClaudeDeck's
`release.published` webhook and auto-bump:

```yaml
# .github/workflows/bump-cask.yml in the homebrew-tap repo
name: Bump cask on ClaudeDeck release
on:
  repository_dispatch:
    types: [claudedeck-released]
  workflow_dispatch:
    inputs:
      version:
        required: true

jobs:
  bump:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Compute hashes
        run: |
          V="${{ github.event.client_payload.version || github.event.inputs.version }}"
          BASE="https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/download/v${V}"
          ARM_SHA=$(curl -sL "${BASE}/claudedeck_${V}_aarch64.dmg" | shasum -a 256 | cut -d' ' -f1)
          INTEL_SHA=$(curl -sL "${BASE}/claudedeck_${V}_x64.dmg" | shasum -a 256 | cut -d' ' -f1)
          sed -i '' \
            -e "s/version \".*\"/version \"${V}\"/" \
            -e "s|arm: .*|arm:   \"${ARM_SHA}\",|" \
            -e "s|intel: .*|intel: \"${INTEL_SHA}\"|" \
            Casks/claudedeck.rb
      - name: Commit
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Casks/claudedeck.rb
          git commit -m "Bump claudedeck to v${{ github.event.client_payload.version }}"
          git push
```

And in ClaudeDeck's `release.yml`, after the build job, fire the dispatch:

```yaml
- name: Notify homebrew-tap
  if: success()
  run: |
    gh api repos/raphaelbarbosaqwerty/homebrew-tap/dispatches \
      --field "event_type=claudedeck-released" \
      --field "client_payload[version]=${GITHUB_REF_NAME#v}"
  env:
    GH_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
```

(That requires a personal access token with `repo` scope on the tap.)

## TL;DR

For the first publish, do steps 1–4 manually. Automate step 5 once you're
shipping releases regularly enough to get tired of the manual sha256 dance.
