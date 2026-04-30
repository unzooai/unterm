# Unterm

Unterm is a Windows-first terminal built from a customized WezTerm engine. It focuses on a practical developer workflow: fast terminal sessions, project-aware tabs, built-in screenshots, proxy switching, theme switching, CLI access, and an MCP server surface for automation.

## Current Release

Download the latest Windows installer from GitHub Releases:

https://github.com/unzooai/unterm/releases

Recommended package:

```text
Unterm-0.2.1-x64.msi
```

The MSI installs Unterm into `Program Files\Unterm` and creates a Start Menu shortcut.

## Features

- Native terminal window based on the WezTerm rendering and PTY stack.
- Compact bottom status bar for project directory, proxy state, theme, and screenshots.
- Screenshot modes:
  - exclude the current Unterm window
  - include the current Unterm window
- Project directory workflow:
  - status bar shows the active project directory
  - directory picker opens a new tab in the selected project
- Proxy controls:
  - status bar toggle
  - proxy settings overlay
  - persisted proxy configuration for new shells
- Theme controls:
  - quick theme cycling
  - theme selector overlay
  - immediate palette apply without full app reload
- CLI support through `unterm.exe`.
- MCP server support for automation tools.

## Basic Usage

Start Unterm from the Start Menu after installing the MSI, or run it directly:

```powershell
unterm.exe start
```

Check the CLI:

```powershell
unterm.exe --help
```

## Configuration

Unterm reads user configuration from:

```text
%USERPROFILE%\.unterm\
```

Current persisted product settings include:

- `proxy.json`
- `theme.json`

## Development

Build the main Windows executable:

```powershell
cargo build -p unterm
```

Build the MSI after creating or refreshing the release runtime directory:

```powershell
D:\code\unterm\.tools\wix.exe build D:\code\unterm\installer\Unterm.wxs -d SourceDir=D:\code\unterm-release-stage\unterm -arch x64 -o D:\code\unterm-release-stage\Unterm-0.2.1-x64.msi
```

## Repository

This repository is the main Unterm project:

https://github.com/unzooai/unterm

Unterm includes modified WezTerm components. Upstream WezTerm remains a separate project by Wez Furlong and contributors.
