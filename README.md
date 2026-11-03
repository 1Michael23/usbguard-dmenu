# usbguard-dmenu

A small program to interface between **usbguard** and any **dmenu-compatible launcher**
(fuzzel, bemenu, rofi, dmenu, vicinae, tofi, wofi, etc.).

When a new USB device is blocked by usbguard, press your bound key to open a
selection menu, pick the device, and choose whether to **Allow**,
**Allow permanently**, **Block**, or **Reject** it.

## Requirements

- [usbguard](https://usbguard.github.io/) daemon running
- A dmenu-compatible launcher on `$PATH`
- Your user must have usbguard IPC listen privileges:
  ```bash
  sudo usbguard add-user $USER -d listen
  ```

## Usage

The launcher and its flags are passed as command-line arguments. The prompt
text is appended automatically.

```
usbguard-dmenu [LAUNCHER] [FLAGS...]
```

Defaults to `fuzzel --dmenu --prompt` if no arguments are given.

### Examples

```bash
usbguard-dmenu                                    # default: fuzzel
usbguard-dmenu bemenu -p                          # bemenu
usbguard-dmenu rofi -dmenu -p                     # rofi in dmenu mode
usbguard-dmenu vicinae dmenu --placeholder        # vicinae
usbguard-dmenu dmenu -p                           # vanilla dmenu
usbguard-dmenu wofi --dmenu --prompt              # wofi
usbguard-dmenu tofi --prompt-text                 # tofi
```

### Bind a key in your compositor

```
bindsym $mod+u exec usbguard-dmenu
```

Or with a specific launcher:

```
bindsym $mod+u exec usbguard-dmenu rofi -dmenu -p
```

## Install with Nix

Add as a flake input, then install the package:

```nix
# flake.nix
inputs.usbguard-dmenu.url = "github:1Michael23/usbguard-dmenu";
```

```nix
# configuration.nix or home.nix
environment.systemPackages = [ inputs.usbguard-dmenu.packages.${pkgs.system}.default ];
```
