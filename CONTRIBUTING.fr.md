# Contribuer à Dot

[English](CONTRIBUTING.md) · **Français** · [Español](CONTRIBUTING.es.md) · [Português](CONTRIBUTING.pt.md)

Merci de ton intérêt. Un bug ou une idée : ouvre une issue avec son formulaire. Une faille de
sécurité : jamais en public, voir [`SECURITY.fr.md`](SECURITY.fr.md).

## Avant d'écrire du code

Pour un changement plus gros qu'un correctif, ouvre d'abord une issue pour en parler. Une pull
request petite et centrée sur une seule chose se relit et se fusionne bien plus vite.

## Construire et vérifier

Les prérequis et la construction sont dans le [`README`](README.fr.md). Avant d'ouvrir une pull
request :

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
```

La CI lance les mêmes vérifications sur macOS et Linux.

## Ce qu'on attend d'un changement

- **Des tests** pour toute logique, dans `tests/<crate>/`, au même chemin que le module testé, et
  qui prouvent les deux sens : l'entrée valide passe, l'invalide est refusée.
- **Aucune action de Claude sans permission**, aucun chemin hors du dossier du projet, aucun secret
  dans un journal ou un message d'erreur.
- **Mesuré, pas supposé** : un changement sur un chemin chaud (flux du modèle, rendu de l'interface,
  démarrage) donne ses chiffres avant et après (`cargo bench`).
- **Aucun texte affiché écrit dans le code** : chaque texte passe par `dot-i18n`, traduit dans
  chaque langue (`crates/dot-i18n/locales/` : anglais, français, espagnol, portugais du Brésil). Les
  tests échouent si un texte manque dans l'une d'elles.
- **Des commits [Conventional Commits](https://www.conventionalcommits.org/)** : `feat(desktop): …`,
  `fix(model): …`, description courte à l'impératif, en anglais.

En proposant une contribution, tu acceptes qu'elle soit publiée sous la licence du projet,
[Apache 2.0](LICENSE).
