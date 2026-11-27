## Installer

**Mac (Apple silicon)** : décompresser `Dot-{version}-macos-arm64.zip`, puis glisser `Dot.app` dans Applications. Cette pré-version n'est pas signée par Apple : au premier lancement, macOS refuse de l'ouvrir. Pour l'autoriser, ouvrir Réglages Système › Confidentialité et sécurité › « Ouvrir quand même », ou lancer une fois dans un terminal :

```bash
xattr -dr com.apple.quarantine /Applications/Dot.app
```

macOS redemande aussi l'accès au trousseau à la première connexion de chaque nouvelle version.

**Linux (x86_64 ou arm64, glibc 2.35 ou plus récente, comme Ubuntu 22.04 et Debian 12)** : prendre le paquet de son processeur (`uname -m` : `x86_64` ou `aarch64`), puis :

```bash
tar -xzf dot-{version}-x86_64-unknown-linux-gnu.tar.gz
./dot-{version}-x86_64-unknown-linux-gnu/install.sh
```

(`aarch64` à la place de `x86_64` sur arm64.)

Dot s'installe dans `~/.local/share/dot-app`, la commande `dot` dans `~/.local/bin`, avec son entrée dans le lanceur d'applications.

**Vérifier un téléchargement** : `shasum -a 256 -c SHA256SUMS --ignore-missing`.

`Dot-{version}-macos-arm64.dSYM.zip` contient les symboles de débogage, pour lire la pile d'un plantage du journal : inutile pour se servir de Dot.
