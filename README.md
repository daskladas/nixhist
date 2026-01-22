# nixhist

**A beautiful TUI for viewing, comparing, and managing NixOS generations.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![NixOS](https://img.shields.io/badge/NixOS-24.11+-5277C3.svg)](https://nixos.org/)

---

## Overview

`nixhist` is a terminal user interface (TUI) for NixOS that helps you:

- **View** all System and Home-Manager generations at a glance
- **Compare** packages between any two generations
- **Restore** to a previous generation with one keypress
- **Delete** old generations safely (with 10-second undo window)
- **Pin** important generations to protect them from deletion

Built with Rust for speed and reliability. Supports both Flakes and traditional Channels setups.

---

## Screenshots

```
┌─ nixhist · thinkpad ─────────────────────────────────────────────────────────┐
│  [1] Overview │ [2] Packages │ [3] Diff │ [4] Manage │ [5] Settings          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─ System (142) ─────────────────┐  ┌─ Home-Manager (89) ───────────────┐  │
│  │                                │  │                                   │  │
│  │  ● #142  22.01.26 14:32  24.11 │  │  ● #89  22.01.26 14:35  stable   │  │
│  │    #141  21.01.26 09:15  24.11 │  │    #88  20.01.26 18:22  stable   │  │
│  │  ★ #140  18.01.26 11:03  24.11 │  │    #87  18.01.26 11:05  stable   │  │
│  │    #139  15.01.26 08:44  24.11 │  │  ★ #86  15.01.26 08:45  stable   │  │
│  │    #138  12.01.26 16:20  24.11 │  │    #85  12.01.26 16:22  stable   │  │
│  │                                │  │                                   │  │
│  └────────────────────────────────┘  └───────────────────────────────────┘  │
│                                                                              │
│  24.11.20250115 · 6.6.52 · 1847 pkgs · 12.4 GB                              │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│  [j/k] Navigate  [Tab] Switch Panel  [Enter] View Packages  [q] Quit        │
└──────────────────────────────────────────────────────────────────────────────┘

Legend: ● = current   ★ = pinned   ⚡ = in bootloader
```

---

## Features

| Feature | Description |
|---------|-------------|
| **5 Tabs** | Overview, Packages, Diff, Manage, Settings |
| **System + Home-Manager** | Supports standalone and NixOS module installations |
| **Flakes & Channels** | Works with both configuration styles |
| **3 Themes** | Gruvbox (default), Nord, Transparent |
| **Responsive Layout** | Auto-switches between side-by-side and tabs based on terminal width |
| **Safe Operations** | Confirmation dialogs, 10s undo timer, pin protection |
| **Dry-Run Mode** | Preview what would happen without making changes |
| **Boot Entry Info** | Shows which generations are in your bootloader |

---

## Installation

### Using Nix Flakes (Recommended)

```bash
# Run directly without installing
nix run github:daskladas/nixhist

# Or install to your profile
nix profile install github:daskladas/nixhist
```

### Add to NixOS Configuration

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixhist.url = "github:daskladas/nixhist";
  };

  outputs = { self, nixpkgs, nixhist, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ({ pkgs, ... }: {
          environment.systemPackages = [
            nixhist.packages.${pkgs.system}.default
          ];
        })
      ];
    };
  };
}
```

### Build from Source

```bash
git clone https://github.com/daskladas/nixhist
cd nixhist
cargo build --release
./target/release/nixhist
```

---

## Usage

```bash
nixhist              # Normal mode
nixhist --dry-run    # Preview mode (no changes made)
nixhist --help       # Show help
nixhist --version    # Show version
```

### Keybindings

| Key | Action |
|-----|--------|
| `1-5` | Switch tabs |
| `j` / `k` | Navigate down / up |
| `g` / `G` | Jump to top / bottom |
| `Tab` | Switch panel / focus |
| `Enter` | Select / confirm |
| `Space` | Toggle selection (Manage tab) |
| `/` | Filter packages (Packages tab) |
| `R` | Restore generation |
| `D` | Delete generation(s) |
| `P` | Pin / unpin generation |
| `Esc` | Cancel / close popup |
| `q` | Quit |

### Tabs

1. **Overview** — View System and Home-Manager generations side-by-side
2. **Packages** — Browse packages in a selected generation with filter
3. **Diff** — Compare packages between any two generations
4. **Manage** — Restore, delete, or pin generations
5. **Settings** — Configure theme, layout, and display options

---

## Configuration

Configuration is stored in `~/.config/nixhist/config.toml`:

```toml
theme = "gruvbox"      # gruvbox | nord | transparent
layout = "auto"        # auto | sidebyside | tabsonly

[display]
show_nixos_version = true
show_kernel_version = true
show_package_count = true
show_size = true
show_store_path = false
show_boot_entry = true

[pinned]
system = [140, 130, 100]
home_manager = [85, 70]
```

---

## Safety Features

| Feature | Description |
|---------|-------------|
| **10-Second Undo** | After deletion, you have 10 seconds to press `U` to cancel |
| **Pin Protection** | Pinned generations cannot be deleted (unpin first) |
| **Current Protection** | The current/active generation cannot be deleted |
| **Confirmation Dialogs** | All destructive actions require confirmation |
| **Dry-Run Mode** | Test commands without executing them |
| **Command Preview** | See the exact command before it runs |

---

## Compatibility

| Configuration | Status |
|---------------|--------|
| NixOS with Flakes | ✅ Supported |
| NixOS with Channels | ✅ Supported |
| Home-Manager (standalone) | ✅ Supported |
| Home-Manager (NixOS module) | ✅ Supported |
| systemd-boot | ✅ Supported |
| GRUB | ✅ Supported |

---

## Roadmap

### v1.0.0 (Current)

- [x] View System and Home-Manager generations
- [x] Package list with filter
- [x] Diff between two generations
- [x] Restore to previous generation
- [x] Delete generations (single and multi-select)
- [x] Pin generations
- [x] 3 built-in themes
- [x] Responsive layout
- [x] Dry-run mode
- [x] Boot entry indicator

### v1.1.0 (Planned)

- [ ] Quick-delete: "Delete all older than X days (except pinned)"
- [ ] Quick-delete: "Keep last N generations"
- [ ] Package history: Track package across all generations
- [ ] Export package list to file

### v1.2.0 (Planned)

- [ ] Garbage collection integration (`nix-collect-garbage`)
- [ ] Disk space analysis per generation
- [ ] Search across all tabs
- [ ] Custom keybinding configuration

### v2.0.0 (Future)

- [ ] Multi-machine support (view generations from remote hosts)
- [ ] Rollback scheduling (schedule a rollback after testing)
- [ ] Integration with nixos-rebuild

---

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## Troubleshooting

### "Permission denied" when restoring/deleting

System generations require `sudo`. The app will prompt for your password.

### Home-Manager not detected

Make sure Home-Manager is properly installed. Check if one of these paths exists:
- `~/.local/state/home-manager/profiles/` (standalone)
- `/nix/var/nix/profiles/per-user/$USER/home-manager` (module)

### Generations not loading

Ensure `nix-env` is in your PATH and you have read access to `/nix/var/nix/profiles/`.

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- Built with [ratatui](https://github.com/ratatui-org/ratatui) — the Rust TUI library
- Inspired by [lazygit](https://github.com/jesseduffield/lazygit) and [btop](https://github.com/aristocratos/btop)
- Thanks to the NixOS community for the amazing package manager
