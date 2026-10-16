# Privacy Policy

_Last updated: October 16, 2026_

Jibaru Editor ("the software") is a free, open-source desktop video editor
for macOS and Windows. This policy covers the desktop app, the `jb`
command-line tool and the `jb-mcp` server distributed from
<https://github.com/Jibaru/jibaru-editor>.

## Summary

**Jibaru Editor does not collect, store or transmit any personal data.** It
has no accounts, no telemetry, no analytics, no crash reporting and no
advertising. Your projects and media never leave your computer unless you
move them yourself.

## Data that stays on your computer

The software reads and writes these files locally:

- **Your projects and media**: the files you open, import, save or export.
- **Caches**: thumbnails, waveforms and preview proxies, stored in your
  user cache folder (`~/Library/Caches/jibaru-editor` on macOS,
  `%LOCALAPPDATA%\jibaru-editor\cache` on Windows). You can delete them at any
  time; they are rebuilt when needed.
- **Autosaves, backups and settings**: stored in the app's data folder for
  your user account.

None of this is sent anywhere.

## Network connections

The software only connects to the internet in these cases:

- **Update checks.** The desktop app checks for new versions at startup
  and every 24 hours by downloading a small manifest from GitHub
  (`github.com/Jibaru/jibaru-editor/releases`). If you choose to install
  an update, the installer is downloaded from the same place. This is a
  plain download: no identifier, project information or usage data is
  sent. GitHub may log the request (for example your IP address) under
  [GitHub's privacy statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement).
  You can turn automatic update checks off with **Help → Check for Updates
  Automatically** (also in the update notice).
- **Links you click.** Help menu items open web pages in your default
  browser.
- **Agents you connect.** The `jb-mcp` server talks only to the AI client
  that starts it on your computer, over standard input and output. It
  opens no network ports. Whatever that client does with project
  information is governed by the client's own privacy policy.

## Children's privacy

The software does not collect data from anyone, including children.

## Changes

Changes to this policy are published in this file in the repository; the
history of changes is visible in its commit log.

## Contact

Questions about this policy can be asked by opening an issue at
<https://github.com/Jibaru/jibaru-editor/issues>.
