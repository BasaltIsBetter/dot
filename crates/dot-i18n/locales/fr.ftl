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
sign-in-anthropic-account = Compte Anthropic
sign-in-anthropic-account-detail = facturé à l’usage, Console Anthropic, via ant
sign-in-api-key = Clé API
sign-in-api-key-detail = facturée à l’usage, Console Anthropic
sign-in-choose-hint = ↑↓ choisir · entrée valider
sign-in-key-prompt = Colle ta clé API Anthropic.
sign-in-key-hint = cmd+V coller · entrée connecter · échap retour
sign-in-checking-key = Vérification de la clé auprès d’Anthropic…
sign-in-waiting-for-claude-code = Connexion dans ton navigateur, via Claude Code…
sign-in-waiting-for-ant = Connexion dans ton navigateur, via ant…
sign-in-cancel-hint = échap annuler
sign-in-key-malformed = Ce n’est pas une clé API Anthropic : elle commence par sk-ant-.
sign-in-key-rejected = Anthropic a refusé cette clé.
sign-in-anthropic-unreachable = Anthropic ne répond pas. Vérifie ta connexion, puis réessaie.
sign-in-claude-code-missing = Claude Code n’est pas installé. Installe-le, puis réessaie.
sign-in-claude-code-failed = La connexion via Claude Code n’a pas abouti.
sign-in-ant-missing = L’outil ant d’Anthropic n’est pas installé : brew install anthropics/tap/ant
sign-in-account-failed = La connexion au compte Anthropic via ant n’a pas abouti.
sign-in-not-connected = { -app-name } n’est plus connecté à Claude.
sign-in-storage-failed = Le trousseau de macOS a refusé l’accès.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5 h
status-weekly-limit = 7 j
status-effort = effort
effort-low = faible
effort-medium = moyen
effort-high = élevé
effort-extra-high = très élevé
effort-max = max

## Prompter, above the status bar

prompter-placeholder = Écris à Claude · entrée envoyer · maj+entrée nouvelle ligne
prompter-replying = Claude répond · échap arrêter

## Images, in the prompter and the conversation

image-label = image
image-pasted = image collée
size-kilobytes = Ko
size-megabytes = Mo
image-unsupported = Ce n’est pas une image que Claude lit : PNG, JPEG, GIF ou WebP.
image-too-large = Cette image dépasse 5 Mo, la limite de Claude.

## Conversation, when a reply does not end as Claude finished it

reply-stopped = Réponse arrêtée.
reply-unauthorized = Anthropic a refusé la clé ou le compte.
reply-rejected = Anthropic a refusé la demande.
reply-rate-limited = Limite d’utilisation atteinte. Réessaie plus tard.
reply-overloaded = Claude est surchargé. Réessaie dans un instant.
reply-unreachable = Anthropic ne répond pas. Vérifie ta connexion, puis réessaie.
reply-declined = Claude a refusé de répondre à cette demande.
reply-claude-code-failed = Claude Code n’a pas pu répondre.

## Conversation, the tools Claude uses: what each one does, in front of what it works on

tool-read = lire
tool-write = écrire
tool-edit = modifier
tool-run = $
tool-find-files = trouver
tool-search-text = chercher
tool-fetch = ouvrir
tool-search-web = web
tool-delegate = déléguer
tool-other = outil

## Conversation, how a tool ended, after what it worked on

tool-failed = échec
tool-refused = refusé
tool-stopped = arrêté
tool-lines = { $count ->
    [one] ligne
   *[other] lignes
}
tool-files = { $count ->
    [one] fichier
   *[other] fichiers
}
tool-matches = { $count ->
    [one] résultat
   *[other] résultats
}
tool-exit = code

## Prompter, when Claude asks before it goes ahead

permission-run = Lancer cette commande ?
permission-change = Modifier ce fichier ?
permission-change-outside = Modifier ce fichier, hors du projet ?
permission-read-outside = Lire ceci, hors du projet ?
permission-fetch = Ouvrir cette page ?
permission-search-web = Chercher sur le web ?
permission-use = Utiliser cet outil ?
permission-allow = entrée autoriser
permission-deny = échap refuser
grant-command = tab toujours cette commande
grant-changes = tab tous les fichiers du projet
grant-file = tab toujours celui-ci
grant-host = tab toujours ce site
grant-search-web = tab toujours le web
grant-tool = tab toujours cet outil
