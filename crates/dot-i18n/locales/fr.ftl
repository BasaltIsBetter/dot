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
sign-in-crashed = { -app-name } a planté pendant la connexion. Réessaie.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5 h
status-weekly-limit = 7 j
status-base = socle
effort-low = faible
effort-medium = moyen
effort-high = élevé
effort-extra-high = très élevé
effort-max = max

## Prompter, above the status bar

suggestion-explain-project = Explique-moi ce projet
suggestion-fix-bug = Trouve et corrige un bug
suggestion-write-tests = Écris les tests qui manquent
suggestion-review-changes = Relis mes derniers changements
suggestion-simplify-code = Rends ce code plus simple
suggestion-find-startup = Où se fait le démarrage de l'app ?
suggestion-summarize-commits = Résume les derniers commits
suggestion-run-tests = Lance les tests et répare ce qui casse

## Key help, beside the reading line while nothing happens

help-send-key = entrée
help-send = envoyer
help-new-line-key = maj+entrée
help-new-line = aller à la ligne
help-pause-key = tab
help-pause = pause
help-stop-key = échap
help-stop = arrêter
help-review-key = molette
help-review = relire
help-switch-key = ⌘↑ ⌘↓
help-switch = changer de session
help-settings-key = ⌘ molette
help-settings = réglages

## Images, in the prompter and the conversation

image-label = image
image-pasted = image collée
size-kilobytes = Ko
size-megabytes = Mo
decimal-separator = ,
unit-seconds = s
instrument-tokens = Jetons
instrument-time = Durée
instrument-agents = Agents
unit-tokens-per-second = j/s
unit-tokens = { $count ->
    [one] jeton
   *[other] jetons
}
state-thinking = Claude réfléchit
state-writing = Claude écrit
state-reading = Claude lit { $value }
state-searching = Claude cherche { $value }
state-editing = Claude modifie { $value }
state-creating = Claude crée { $value }
state-running = Claude lance { $value }
state-browsing = Claude consulte { $value }
state-using = Claude utilise { $value }
state-waiting-for-agents = { $count ->
    [one] Claude attend 1 agent
   *[other] Claude attend { $count } agents
}
percent = { $value }{"\u00a0"}%
state-paused = En pause
state-queued = 1 prompt en attente
note-placeholder = une indication pour Claude, puis entrée · tab reprend
queued-after-the-reply = envoyé après la réponse
agents-at-work = { $count ->
    [one] 1 agent au travail
   *[other] { $count } agents en parallèle
}
agents-out-of-sight = { $count ->
    [one] 1 agent
   *[other] { $count } agents
}
agent-started-by-claude = lancé par Claude · { $value }
agent-view-placeholder = un agent au travail ne lit rien de ce qu’on écrit · échap revient
state-agent-working = { $value } · travaille
state-agent-done = { $value } · a fini
agent-read = { $count ->
    [one] 1 fichier lu
   *[other] { $count } fichiers lus
}
agent-searched = { $count ->
    [one] 1 recherche
   *[other] { $count } recherches
}
agent-edited = { $count ->
    [one] 1 fichier modifié
   *[other] { $count } fichiers modifiés
}
agent-created = { $count ->
    [one] 1 fichier créé
   *[other] { $count } fichiers créés
}
agent-ran = { $count ->
    [one] 1 commande
   *[other] { $count } commandes
}
agent-browsed = { $count ->
    [one] 1 page lue
   *[other] { $count } pages lues
}
agent-delegated = { $count ->
    [one] 1 agent lancé
   *[other] { $count } agents lancés
}
agent-used = { $count ->
    [one] 1 outil
   *[other] { $count } outils
}
session-name = session { $value }
cost-dollars = { $value }{"\u00a0"}$
quote-open = «{"\u00a0"}
quote-close = {"\u00a0"}»
image-unsupported = Ce n’est pas une image que Claude lit : PNG, JPEG, GIF ou WebP.
image-too-large = Cette image dépasse 5 Mo, la limite de Claude.
image-too-many = Un envoi joint 20 images au plus, 20 Mo en tout.

## Conversation, when a reply does not end as Claude finished it

reply-stopped = Réponse arrêtée.
reply-unauthorized = Anthropic a refusé la clé ou le compte.
reply-rejected = Anthropic a refusé la demande. Si la conversation est trop longue, cmd+N en commence une nouvelle.
reply-rate-limited = Limite d’utilisation atteinte. Réessaie plus tard.
reply-overloaded = Claude est surchargé. Réessaie dans un instant.
reply-unreachable = Anthropic ne répond pas. Vérifie ta connexion, puis réessaie.
reply-declined = Claude a refusé de répondre à cette demande.
reply-claude-code-failed = Claude Code n’a pas pu répondre.
reply-oversized = Une partie de la réponse dépasse 4 Mo, plus que { -app-name } ne lit : la réponse est coupée et rien n’en est gardé.
reply-crashed = { -app-name } a planté pendant cette réponse : la conversation repart de zéro.
conversation-restarted = Nouvelle conversation : Claude ne voit plus ce qui précède.
conversation-compacted = Conversation résumée pour Claude :

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
gutter-lines = l.
code-new = nouveau
code-fold = replier
editor-by-claude = Claude :
editor-created-by-claude = créé par Claude ·
editor-by-you = toi :
editor-to-claude = la version de Claude
editor-to-before = avant Claude
editor-to-yours = tes retouches
editor-back = revenir
editor-saved = enregistré à chaque frappe
editor-not-saved = pas enregistré
editor-outside = Ce fichier est hors du dossier de la session : Dot ne l’ouvre pas.
editor-not-a-file = Ce n’est pas un fichier.
editor-too-large = Ce fichier dépasse 1 Mio : trop grand pour être retouché ici.
editor-not-text = Ce fichier n’est pas du texte.
editor-failed = Ce fichier ne s’ouvre pas.
editor-lost = Dot n’a pas pu enregistrer { $value } : tes dernières retouches n’y sont pas.

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

## Settings
settings-title = Réglages
settings-saved = enregistré à chaque changement
settings-unsaved = Les réglages n'ont pas pu être enregistrés.
settings-choose = choisir
settings-change = changer
settings-reset = défaut
settings-close = fermer
settings-default = Défaut
settings-rank = Réglage
settings-conversation = Conversation
settings-display = Affichage
setting-model = Modèle
setting-model-hint = Le modèle qui répond, à partir du prochain envoi.
setting-effort = Effort
setting-effort-hint = Combien Claude réfléchit avant de répondre. Plus haut : plus lent, plus cher.
setting-summaries = Résumé automatique
setting-summaries-hint = Dot résume la conversation quand la relire coûte plus qu'un résumé.
summaries-when-they-pay = quand il paie
summaries-left-to-claude = contexte plein
setting-permissions = Actions de Claude
setting-permissions-hint = Lire, écrire, lancer une commande : sans question, ou une question à chaque fois.
permissions-bypass = accordées d'office
permissions-ask = demandées
setting-color = Couleur
setting-color-hint = La couleur de cette session : ce qui change en ce moment, sa vignette, les outils et la jauge. Chaque nouvelle session prend une couleur qu’aucune autre n’a.
hue-orange = orange
hue-red = rouge
hue-amber = ambre
hue-green = vert
hue-cyan = cyan
hue-blue = bleu
hue-violet = violet
hue-pink = rose
setting-language = Langue
setting-language-hint = La langue de tous les textes de Dot. Claude répond dans celle de ta question.
setting-column = Largeur de colonne
setting-column-hint = Le nombre de caractères par ligne, au plus. Une fenêtre étroite en met moins.
column-characters = { $value } col
setting-reading-line = Ligne de lecture
setting-reading-line-hint = La hauteur de la bande où l'on écrit. Elle bouge tout de suite.
reading-line-middle = au centre
reading-line-third = au tiers
setting-caret = Curseur _
setting-caret-hint = Le _ de la ligne d'écriture et de la réponse.
caret-blinking = clignote
caret-steady = fixe
setting-sessions-column = Colonne des sessions
setting-sessions-column-hint = auto : affichée dès que la fenêtre est assez large.
sessions-column-auto = auto
sessions-column-never = jamais
setting-state-colors = Couleurs des états
setting-state-colors-hint = ✓ en vert, ✗ en rouge, arrêté en ambre ; la jauge du contexte dans la couleur de la session quand elle monte, et dès 80 %.
setting-tool-colors = Couleurs des outils
setting-tool-colors-hint = Le glyphe de chaque outil (▤ ⌕ ✎ ⌘ ⧉), dans la couleur de la session.
toggle-on = oui
toggle-off = non
setting-size = Taille
setting-size-hint = Taille de tout ce que Dot dessine : texte, marges, barre d'état, réglages ; 80 % était la taille d'avant.
size-percent = { $value } %
settings-outputs = Sorties des commandes
setting-output-cleaning = Nettoyage du terminal
setting-output-cleaning-hint = Couleurs et lignes redessinées sur place (barres de progression) retirées de ce que Claude lit ; une ligne redessinée est dite une fois.
setting-output-rewriting = Réécritures sans perte
setting-output-rewriting-hint = Chemins répétés écrits une fois, listes de fichiers repliées, JSON sans espaces, arbres, tables, git status et git log plus courts ; rien n'est perdu.
setting-output-progress = Progression comptée
setting-output-progress-hint = Progression des outils reconnus (builds, installations) et tests réussis comptés : [+12 lines: build progress] ; les erreurs restent entières.
setting-contents-cut = Coupe des fichiers et diffs
setting-contents-cut-hint = Un fichier ou un diff plus long que la première taille est ramené vers la seconde, gardé entier dans /tmp/dot-whole ; mesuré : 10 → 6 Ko.
setting-outputs-cut = Coupe des autres sorties
setting-outputs-cut-hint = Une autre sortie plus longue que la première taille est ramenée vers la seconde si la coupe retire 4 Ko au moins, gardée entière dans /tmp/dot-whole ; mesuré : 7 → 3 Ko.
cut-never = jamais
cut-bounds = { $value } Ko
settings-connection = Connexion
setting-folder = Dossier
setting-folder-hint = entrée ouvre le sélecteur de dossier ; une nouvelle session s'ouvre dans celui choisi.
folder-none = aucun
folder-picker-prompt = Ouvrir une session ici
setting-account = Compte
setting-account-hint = entrée change de connexion : abonnement Claude, compte Anthropic ou clé API.
account-none = aucun
setting-api-key = Clé API
setting-api-key-hint = entrée pour en coller une, entrée encore pour la vérifier. Elle va dans le trousseau, jamais en clair.
api-key-none = aucune
api-key-kept = dans le trousseau
api-key-checking = vérification…
setting-sign-out = Se déconnecter
setting-sign-out-hint = entrée, puis entrée encore pour confirmer. Dot oublie sa connexion ; Claude Code et ant restent connectés.
sign-out-confirm = entrée encore pour confirmer
selection-label = Sélection
selection-characters = { $value } car.
selection-lines = { $value } lignes
selection-copy = copier
selection-copied = copié
