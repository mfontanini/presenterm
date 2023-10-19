# Maintainer: pwnwriter < hey@pwnwriter.xyz >

pkgname=presenterm-bin
pkgver=0.2.1
pkgrel=1
pkgdesc="A terminal slideshow tool"
arch=('x86_64')
url="https://github.com/mfontanini/presenterm"
license=('BSD 2-Clause')
source=("$pkgname-$pkgver.tar.gz::$url/releases/download/v$pkgver/presenterm-$pkgver-x86_64-unknown-linux-gnu.tar.gz")
sha512sums=('6f1f76b208b2586aee6ef6884699866ca8f22fc242a7cc83b767fff3e3d8fc22bfebd2fd25151e87faf88fe352758c4877823726dc41ce8390e481038dfa56e5')

build() {
  # Nothing to do here for a binary package
  return 0
}

package() {
  cd "${pkgname%-bin}-$pkgver"
  install -Dm 755 "${pkgname%-bin}" -t "${pkgdir}/usr/bin"
  install -Dm 644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm 644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
