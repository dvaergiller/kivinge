# Kivinge

An unofficial command-line client for [Kivra](https://kivra.se/), the Swedish
digital mailbox service.

**Disclaimer: This project is not affiliated with, supported by, or endorsed by
Kivra in any way. Do not contact Kivra for support or other requests regarding
this client. This client may break at any time if Kivra publishes changes to
their service.**

## Project Status

This project is in early development. There are likely bugs and missing features.
If you encounter any issues or have suggestions, please [open an issue on
GitHub](https://github.com/dvaergiller/kivinge/issues).

## Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- OpenSSL development libraries
- libfuse3 development libraries

On Debian/Ubuntu:

```bash
sudo apt install libssl-dev libfuse3-dev
```

On Arch Linux:

```bash
sudo pacman -S openssl fuse3
```

## Installation

```bash
cargo install --path .
```

### Shell Completions (optional)

Generate shell completions for your shell:

```bash
# Bash
kivinge completions bash > ~/.local/share/bash-completion/completions/kivinge

# Zsh
kivinge completions zsh > ~/.zfunc/_kivinge
```

## Authentication

Kivinge uses BankID for authentication. On first use, you will be prompted to
scan a QR code with the BankID app. The session is saved locally and reused for
subsequent commands.

```bash
kivinge login   # Log in to Kivra
kivinge logout  # Log out and delete saved session
```

## CLI

The CLI provides direct access to your Kivra inbox from the command line.

### Commands

```bash
kivinge list                          # List all items in inbox
kivinge view <item_id>                # View details of an inbox item
kivinge download <item_id> <n> [dir]  # Download attachment n to directory
kivinge open <item_id> <n>            # Open attachment n with default application
```

### Examples

```bash
# List inbox
kivinge list

# View item 5
kivinge view 5

# Download the first attachment from item 5 to current directory
kivinge download 5 0

# Download to specific directory
kivinge download 5 0 ~/Documents

# Open the first attachment from item 5
kivinge open 5 0
```

## TUI

An interactive terminal user interface for browsing your inbox.

```bash
kivinge tui
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `n` / Down | Move down |
| `k` / `p` / Up | Move up |
| `l` / `f` / Enter / Right | Select / Open |
| `h` / `b` / Left | Go back |
| `r` | Mark as read |
| `q` / Esc | Quit |

## FUSE

Mount your Kivra inbox as a read-only filesystem. This allows you to browse and
access your documents using standard file tools.

```bash
kivinge mount ~/kivra
```

The filesystem runs as a background daemon. To unmount:

```bash
umount ~/kivra
```

### Structure

```
~/kivra/
  0002_2024-01-15_Company-Name_Invoice/
    2024-01-15T12:00:00+00:00-0-Company-Name-Invoice.pdf
  0001_2024-01-10_Another-Sender_Document/
    2024-01-10T14:22:11+00:00-0-Another-Sender-Document.pdf
    2024-01-10T14:22:11+00:00-1-Another-Sender-Attachment.pdf
```

Each inbox item becomes a directory containing its attachments.

## License

This project is licensed under the GNU General Public License v3.0 - see the
[LICENSE](LICENSE) file for details.
