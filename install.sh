#!/bin/bash
set -e

cargo build --release

sudo cp target/release/keyboard-locker /usr/local/bin/
sudo cp src/icon.png /usr/share/pixmaps/keyboard-locker.png
sudo cp keyboard-locker.desktop /usr/share/applications/
sudo cp 99-keyboard-locker.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=input --action=change
sudo usermod -aG input $USER

echo "Installation complete."