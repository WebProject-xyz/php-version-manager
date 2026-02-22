# PVM (PHP Version Manager)

[![Build Status](https://github.com/WebProject-xyz/php-version-manager/actions/workflows/release.yml/badge.svg)](https://github.com/WebProject-xyz/php-version-manager/actions/workflows/release.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPL_v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Crates.io Version](https://img.shields.io/crates/v/php-version-manager)](https://crates.io/crates/php-version-manager)

Native, blazing fast, zero-configuration PHP version manager for Arch Linux and other Linux/macOS environments, heavily inspired by [fnm](https://github.com/Schniz/fnm).

PVM uses pre-compiled static PHP CLI binaries from [Static PHP CLI (SPC)](https://dl.static-php.dev/) to completely bypass compilation times and library dependency hell on Linux.

## Features
- 🚀 **Blazing Fast**: Written in Rust natively. Execution means zero overhead compared to Docker wrappers.
- ✨ **Zero Configuration**: Auto-switches PHP versions based on `.php-version` files.
- 📦 **Static Binaries**: No compilation needed. The `pvm install` command instantly downloads self-contained `php` executables containing 95% of common extensions pre-baked.
- 🐘 **Native Composer Support**: Works out of the box with your system's global Composer without any explicit proxy or configuration.
- 🖱️ **Interactive TUI Menus**: Run `pvm` without arguments to launch a master selection menu. Or run commands like `pvm use` / `pvm ls-remote` without parameters to select actions via a visual UI.
- 🏷️ **Smart Aliasing**: Install and use patches cleanly by saying `pvm install 8.4`. PVM dynamically figures out the highest patch (`8.4.18`) underneath the hood. 
- ⚡ **Cached Cloud Resolution**: Quickly check for new versions on `dl.static-php.dev` under lightning-fast 24-hour JSON caching.

## Installation

We provide an automatic install script that detects your platform exactly like `fnm` and downloads the pre-compiled native `pvm` binary directly from GitHub Releases into `~/.local/share/pvm/bin`, and then instructs you how to append the hook to your profile.

**Using a script (macOS/Linux)**

```bash
curl -fsSL https://raw.githubusercontent.com/WebProject-xyz/php-version-manager/main/install.sh | bash
```

**Building from Source**
If you prefer to compile the application from scratch using Rust:

```bash
git clone git@github.com:WebProject-xyz/php-version-manager.git
cd php-version-manager
chmod +x build.sh
./build.sh
```

## Usage

```bash
# Enter the master interactive TUI menu
pvm

# Install a specific PHP version instantly (by minor alias or fully-qualified)
pvm install 8.4

# Install the absolute latest version available
pvm install latest

# Use a version in the current shell
pvm use 8.4

# List all local installed versions alongside their specific aliases
pvm ls

# Interactively view and install available cloud versions 
pvm ls-remote

# Verify the current active version
pvm current
```

### Auto-Switching
If you run `pvm init` or manually create a `.php-version` file in a project directory containing `8.3`, PVM will automatically switch to your best local `8.3.x` patch when you `cd` into that folder.
