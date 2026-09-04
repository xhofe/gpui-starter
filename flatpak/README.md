# Flatpak

Files used to ship on Flathub (`flathub/io.github.xhofe.gpui-starter` once accepted):

- `io.github.xhofe.gpui-starter.yml` — flatpak-builder manifest
- `io.github.xhofe.gpui-starter.desktop` — desktop entry (Icon must equal the app-id;
  `assets/gpui-starter.desktop` is the AppImage variant and stays untouched)
- `io.github.xhofe.gpui-starter.metainfo.xml` — AppStream metadata shown in software
  centers

Regenerate the offline crate mirror after a lockfile change:

```bash
./scripts/gen-flatpak-sources.sh
```

Validate locally:

```bash
appstreamcli validate io.github.xhofe.gpui-starter.metainfo.xml
desktop-file-validate io.github.xhofe.gpui-starter.desktop
```

Build:

```bash
flatpak-builder --user --install --force-clean build-dir io.github.xhofe.gpui-starter.yml
flatpak run io.github.xhofe.gpui-starter
```

First-time Flathub submission: fork `flathub/flathub`, branch off `new-pr`, add
`io.github.xhofe.gpui-starter.yml` + `cargo-sources.json` +
`io.github.xhofe.gpui-starter.metainfo.xml`, open the PR. After acceptance,
releases go to the dedicated `flathub/io.github.xhofe.gpui-starter` repo.
`scripts/submit-flathub.sh` automates pinning the tag and assembling those files.
