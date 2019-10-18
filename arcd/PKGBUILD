# Maintainer: Quentin Boyer <qbsecond@gmail.com>
pkgname=arcd
pkgver=0.1.0
pkgrel=1
makedepends=('rust' 'cargo')
arch=('i686' 'x86_64' 'armv6h' 'armv7h')
pkgdesc="A rust node implementing the ensicoin protocol"

build() {
    return 0
}

package() {
    cargo install --root="$pkgdir" arcd
}
