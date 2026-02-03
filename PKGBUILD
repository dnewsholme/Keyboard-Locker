# Maintainer: Daryl
pkgname=keyboard-locker
pkgver=0.1.1
pkgrel=3
pkgdesc="Lock your keyboard input"
arch=('x86_64')
url=""
license=('unknown')
depends=('glibc' 'gcc-libs' 'libxkbcommon' 'wayland' 'libglvnd')
makedepends=('cargo')
source=()
sha256sums=()

build() {
	cd "$startdir"
	cargo build --release --locked
}

package() {
	cd "$startdir"
	install -Dm755 target/release/keyboard-locker "$pkgdir/usr/bin/keyboard-locker"
	install -Dm644 src/icon.png "$pkgdir/usr/share/pixmaps/keyboard-locker.png"
	install -Dm644 keyboard-locker.desktop "$pkgdir/usr/share/applications/keyboard-locker.desktop"
	install -Dm644 99-keyboard-locker.rules "$pkgdir/usr/lib/udev/rules.d/99-keyboard-locker.rules"
}