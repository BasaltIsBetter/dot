# Dot

Agent de code pour macOS et Linux, dans l'esprit de Claude Code, pensé pour être plus rapide, plus sobre et
plus réfléchi : une fenêtre noire dessinée au GPU, dans le style de
[Spaceship](https://chuanqisun.github.io/spaceship/), où l'on écrit à Claude, qui répond en flux et
travaille dans le dossier du projet avec ses outils (lire, écrire, lancer des commandes, chercher,
le web).

## Prérequis

- macOS sur Apple Silicon ou Intel, avec un GPU Metal ; ou Linux sous Wayland ou X11, avec un pilote
  Vulkan ou OpenGL (`mesa-vulkan-drivers` suffit, sans carte graphique). Sous X11, il faut aussi
  `libxkbcommon-x11`. Une clé API est gardée par le trousseau du bureau (Secret Service : GNOME
  Keyring, KWallet…) ; sans trousseau, comme sous WSL, il faut se connecter par Claude Code ou `ant`.
- Rust : la version est épinglée dans `rust-toolchain.toml` (rustup l'installe tout seul).
- Pour se connecter : un abonnement Claude via [Claude Code](https://claude.com/claude-code), un
  compte Anthropic via `ant`, ou une clé API Anthropic.

## Lancer

```bash
cargo run --release -- ~/mon-projet          # Dot ouvert sur un dossier
./crates/dot/macos/bundle.sh                  # l'app Mac : target/release/bundle/macos/Dot.app
./crates/dot/linux/bundle.sh                  # l'archive Linux : target/release/bundle/linux/*.tar.gz
```

Raccourcis : entrée envoie, maj+entrée va à la ligne, échap arrête la réponse, cmd+V colle du
texte ou une image, cmd+N commence une nouvelle conversation. Sous Linux, ctrl remplace cmd, et
ctrl+Q quitte.

Le journal est écrit dans `~/Library/Logs/Dot/dot.log` sur Mac, dans `~/.local/state/dot/dot.log`
sous Linux (1 Mio au plus, le précédent gardé dans
`dot.log.1`) ; il ne contient jamais de clé ni de jeton.

## Agents dans des conteneurs

Des agents peuvent vivre dans des conteneurs Docker (ou Podman) et être appelés à la demande, depuis
un terminal ou par Claude dans Dot (`@relecteur regarde ce fichier`) : chaque agent garde son
conteneur par projet, réveillé au besoin, et sa mémoire dans un volume à lui.

```bash
cargo build --release -p dot-container             # la commande : target/release/dot-agents
dot-agents new relecteur                           # décrit un agent, à modifier ensuite
dot-agents build                                   # construit l'image, avec Claude Code dedans
dot-agents --folder ~/mon-projet login relecteur   # connecte l'agent à Claude, une seule fois
dot-agents --folder ~/mon-projet call relecteur "Relis le dernier commit"
dot-agents list                                    # les agents et leurs conteneurs
dot-agents --folder ~/mon-projet stop relecteur    # endort l'agent
```

Les agents sont décrits en JSON dans `~/.config/dot/agents/` (`~/Library/Application
Support/Dot/agents/` sur Mac), jamais dans un projet : ouvrir un dépôt inconnu n'en lance aucun.

| Champ | Rôle | Défaut |
|---|---|---|
| `description` | ce que Claude lit pour choisir l'agent | vide |
| `workspace` | accès au dossier du projet : `none`, `read`, `write` | `read` |
| `network` | `bridge`, ou `none` pour un agent sans réseau (qui ne peut donc pas être Claude) | `bridge` |
| `memory`, `cpus`, `processes` | limites du conteneur | `2g`, 2, 512 |
| `minutes` | durée maximale d'un appel | 30 |
| `model`, `instructions` | modèle et consignes de Claude Code dans le conteneur | aucun |
| `command` | programme qui répond à la place de Claude Code (demande sur l'entrée, réponse sur la sortie) | Claude Code |
| `image` | image du conteneur ; elle doit fournir `sh`, `tr`, `grep` et un `/home/agent` que tous peuvent écrire | `dot-agent:latest` |
| `env` | noms des variables transmises à l'agent | aucune |

Le conteneur n'a aucun privilège (`--cap-drop ALL`, `no-new-privileges`), tourne sous l'identité de
l'utilisateur et ne voit que le projet, au même chemin que sur la machine ; le dossier personnel
entier et la racine du disque sont refusés. Dedans, Claude Code ne demande pas de permission : c'est
le conteneur qui borne l'agent. Un agent en `write` peut donc tout changer dans le projet, y compris
`.git/hooks`. Claude peut appeler les agents dans Dot quand `dot-agents` est à côté de `dot` et que
la connexion passe par Claude Code ; pas encore avec une clé API.

## Vérifier

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo deny check
cargo bench -p dot-desktop --features bench    # bancs criterion
```

L'intégration continue (`.github/workflows/ci.yml`) lance les mêmes vérifications à chaque push.

## Organisation

Un crate par responsabilité dans `crates/`, leurs tests à part dans `tests/<crate>/` :

| Crate | Rôle |
|---|---|
| `dot` | le binaire : assemble les composants, tient le journal |
| `dot-api` | contrats et types partagés (conversation, outils, permissions, session, textes) |
| `dot-compact` | raccourcit ce que Claude lit des commandes |
| `dot-container` | les agents qui vivent dans des conteneurs, et leur commande `dot-agents` |
| `dot-desktop` | l'app Mac et Linux : fenêtre, conversation, barre d'état (winit + wgpu) |
| `dot-i18n` | les traductions (Fluent, `crates/dot-i18n/locales/`) |
| `dot-auth` | la connexion à Claude |
| `dot-model` | les réponses de Claude : API Messages ou Claude Code |
| `dot-process` | les programmes que Dot lance (environnement, groupe, arrêt, attente) et la lecture bornée, ligne par ligne, de ce qu'il en lit |
| `dot-session` | la session d'une fenêtre : dossier, branche git, consommation |
| `dot-shell` | remplace le shell des commandes de Claude Code, pour raccourcir ce qu'elles affichent |
| `dot-tool` | les outils de Claude sur la machine |

## Licence

Sous licence [Apache 2.0](LICENSE) ; les crédits sont dans [`NOTICE`](NOTICE).

- Une partie des règles de `dot-compact` est adaptée de [RTK](https://github.com/rtk-ai/rtk)
  (Apache 2.0) : voir [`crates/dot-compact/third-party/rtk/`](crates/dot-compact/third-party/rtk/).
- Le design de l'interface s'inspire de [Spaceship](https://github.com/chuanqisun/spaceship), de Sun
  (MIT) ; aucun code n'en est repris.
- La police JetBrains Mono embarquée est sous licence SIL Open Font License 1.1.
