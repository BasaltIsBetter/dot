### Dot texts in French, the reference language.
### One line per text: `identifier = text`. Identifiers are never translated.
### `{ -app-name }` inserts the app name.

-app-name = Dot

## Window

window-title = { -app-name }

## App menu, in the macOS menu bar

menu-about = À propos de { -app-name }
menu-services = Services
menu-hide = Masquer { -app-name }
menu-hide-others = Masquer les autres
menu-show-all = Tout afficher
menu-quit = Quitter { -app-name }

## Errors that stop Dot

error-event-loop = La boucle d’évènements de { -app-name } n’a pas pu tourner.
error-window = macOS a refusé d’ouvrir la fenêtre de { -app-name }.
error-surface = La fenêtre de { -app-name } n’a pas pu être reliée à la carte graphique.
error-adapter = Aucune carte graphique compatible n’a été trouvée.
error-device = La carte graphique n’a pas pu être ouverte.
error-unsupported-surface = La carte graphique ne sait pas dessiner dans la fenêtre de { -app-name }.
error-surface-lost = La surface de dessin de { -app-name } a été perdue.
error-text = { -app-name } n’a pas pu dessiner son texte.

## Sign-in screen, shown while nothing connects Dot to Claude

sign-in-title = Connexion à Claude
sign-in-intro = Choisis comment { -app-name } utilise Claude.
sign-in-claude-account = Compte Claude
sign-in-claude-account-detail = ton abonnement Pro ou Max, via Claude Code
sign-in-api-key = Clé API
sign-in-api-key-detail = facturée à l’usage, Console Anthropic
sign-in-choose-hint = ↑↓ choisir · entrée valider
sign-in-key-prompt = Colle ta clé API Anthropic.
sign-in-key-hint = cmd+V coller · entrée connecter · échap retour
sign-in-checking-key = Vérification de la clé auprès d’Anthropic…
sign-in-waiting-for-claude-code = Connexion dans ton navigateur, via Claude Code…
sign-in-cancel-hint = échap annuler
sign-in-key-malformed = Ce n’est pas une clé API Anthropic : elle commence par sk-ant-.
sign-in-key-rejected = Anthropic a refusé cette clé.
sign-in-anthropic-unreachable = Anthropic ne répond pas. Vérifie ta connexion, puis réessaie.
sign-in-claude-code-missing = Claude Code n’est pas installé. Installe-le, puis réessaie.
sign-in-claude-code-failed = La connexion via Claude Code n’a pas abouti.
sign-in-storage-failed = Le trousseau de macOS a refusé l’accès.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5 h
status-weekly-limit = 7 j
