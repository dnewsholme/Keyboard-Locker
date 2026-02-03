# Keyboard Locker

A simple, effective utility for Linux to lock your keyboard input while keeping your screen visible and the system running.

## Use Cases

*   **Cat Proofing:** Stop your cat from walking across your keyboard and sending unfinished emails, closing windows, or pausing your music.
*   **Disable Laptop Keyboard:** If you are using another keyboard disable the internal laptop one to prevent accidental keypresses.
*   **Child Safety:** Allow your toddler to watch videos or look at photos without worrying about them accidentally deleting files or stopping playback.
*   **Cleaning:** Wipe down your keyboard without needing to shut down your computer or unplug the device.

## Features

*   **System-Level Lock:** Uses `evdev` to grab the keyboard device directly, ensuring input is blocked for all applications.
*   **Configurable Unlock:** Set your own unlock key combination (Default: `CTRL + Q`).
*   **Visual Indicator:** Clear GUI feedback showing when the keyboard is locked.
*   **Wayland & X11 Support:** Works independently of the display server by interacting with kernel input devices.

## Installation

### Arch Linux

A `PKGBUILD` is provided for easy installation on Arch Linux.

```bash
makepkg -si
```

reload the udev rules after install

``` bash
sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=input --action=change 
```

If you aren't in the input group add your user

```bash
sudo usermod -aG input $USER
```

### Other Linux Distributions (Debian, Ubuntu, Fedora, etc.)

You can build and install the application manually using the provided script.

#### 1. Install Dependencies

You will need the Rust toolchain (`cargo`) and development libraries for Wayland and OpenGL.

**Debian / Ubuntu / Mint:**
```bash
sudo apt update
sudo apt install build-essential libxkbcommon-dev libwayland-dev libglvnd-dev pkg-config libssl-dev policykit-1
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Fedora / RHEL:**
```bash
sudo dnf install gcc libxkbcommon-devel wayland-devel libglvnd-devel openssl-devel polkit
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 2. Build and Install

Clone the repository and run the install script:

```bash
chmod +x install.sh
./install.sh
```

This script will compile the application, install it to `/usr/local/bin`, and install a udev rule to allow running without root.

If you aren't in the input group add your user

```bash
sudo usermod -aG input $USER
```

## Usage

1.  Open your application menu and launch **Keyboard Locker**.
2.  The application should open immediately (no password required).
3.  Once the window opens, you can optionally change the unlock key (Default is `Q`).
4.  Click **🔒 LOCK NOW**.
5.  Your keyboard is now locked. To unlock, press `Left Ctrl` + `[Your Configured Key]` (e.g., `Ctrl + Q`).

## Uninstallation

If you installed via the `install.sh` script, you can remove the files manually:

```bash
sudo rm /usr/local/bin/keyboard-locker
sudo rm /usr/share/pixmaps/keyboard-locker.png
sudo rm /usr/share/applications/keyboard-locker.desktop
sudo rm /etc/udev/rules.d/99-keyboard-locker.rules
```