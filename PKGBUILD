pkgname=puff
pkgver=2.0.0
pkgrel=1
license=("custom")
makedepends=('rust' 'cargo')
arch=('i686' 'x86_64' 'armv6h' 'armv7h')
pkgdesc="puff - a tool for managing c/c++ packages"

source=("puff")

package() {
    mkdir -p "${pkgdir}/usr/bin"
    cp "${srcdir}/puff" "${pkgdir}/usr/bin/puff"
    chmod +x "${pkgdir}/usr/bin/puff"
}
