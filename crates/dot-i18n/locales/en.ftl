### Dot texts in English, the reference language.
### One line per text: `identifier = text`. Identifiers are never translated.
### `{ -app-name }` inserts the app name.

-app-name = Dot

## Window

window-title = { -app-name }

## App menu, in the macOS menu bar

menu-about = About { -app-name }
menu-services = Services
menu-hide = Hide { -app-name }
menu-hide-others = Hide Others
menu-show-all = Show All
menu-quit = Quit { -app-name }

## Errors that stop Dot

error-event-loop = The { -app-name } event loop could not run.
error-window = { -system } refused to open the { -app-name } window.
error-surface = The { -app-name } window could not be connected to the graphics card.
error-adapter = No compatible graphics card was found.
error-device = The graphics card could not be opened.
error-unsupported-surface = The graphics card cannot draw in the { -app-name } window.
error-surface-lost = The { -app-name } drawing surface was lost.
error-text = { -app-name } could not draw its text.

## Sign-in screen, shown while nothing connects Dot to Claude

sign-in-title = Connect to Claude
sign-in-intro = Choose how { -app-name } uses Claude.
sign-in-claude-account = Claude account
sign-in-claude-account-detail = your Pro or Max plan, through Claude Code
sign-in-anthropic-account = Anthropic account
sign-in-anthropic-account-detail = billed per use, Anthropic Console, through ant
sign-in-api-key = API key
sign-in-api-key-detail = billed per use, Anthropic Console
sign-in-choose-hint = ↑↓ choose · return confirm
sign-in-key-prompt = Paste your Anthropic API key.
sign-in-key-hint = { -command-key }+V paste · return connect · esc back
sign-in-checking-key = Checking the key with Anthropic…
sign-in-waiting-for-claude-code = Signing in in your browser, through Claude Code…
sign-in-waiting-for-ant = Signing in in your browser, through ant…
sign-in-cancel-hint = esc cancel
sign-in-key-malformed = This is not an Anthropic API key: it starts with sk-ant-.
sign-in-key-rejected = Anthropic rejected this key.
sign-in-anthropic-unreachable = Anthropic is not answering. Check your connection, then try again.
sign-in-claude-code-missing = Claude Code is not installed. Install it, then try again.
sign-in-claude-code-failed = Signing in through Claude Code did not complete.
sign-in-ant-missing = Anthropic’s ant tool is not installed: brew install anthropics/tap/ant
sign-in-account-failed = Signing in to the Anthropic account through ant did not complete.
sign-in-not-connected = { -app-name } is no longer connected to Claude.
sign-in-storage-failed = The { -system } keychain refused access.
sign-in-crashed = { -app-name } crashed while connecting. Try again.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5h
status-weekly-limit = 7d
status-base = base
effort-low = low
effort-medium = medium
effort-high = high
effort-extra-high = extra high
effort-max = max

## Prompter, above the status bar

suggestion-explain-project = Explain this project to me
suggestion-fix-bug = Find and fix a bug
suggestion-write-tests = Write the missing tests
suggestion-review-changes = Review my latest changes
suggestion-simplify-code = Make this code simpler
suggestion-find-startup = Where does the app start up?
suggestion-summarize-commits = Summarize the recent commits
suggestion-run-tests = Run the tests and fix what breaks

## Key help, beside the reading line while nothing happens

help-send-key = return
help-send = send
help-new-line-key = shift+return
help-new-line = new line
help-pause-key = tab
help-pause = pause
help-stop-key = esc
help-stop = stop
help-review-key = wheel
help-review = read again
help-switch-key = { -command-sign }↑ { -command-sign }↓
help-switch = switch session
help-settings-key = { -command-sign } wheel
help-settings = settings

## Images, in the prompter and the conversation

image-label = image
image-pasted = pasted image
size-kilobytes = KB
size-megabytes = MB
decimal-separator = .
unit-seconds = s
instrument-tokens = Tokens
instrument-time = Time
instrument-agents = Agents
unit-tokens-per-second = tok/s
unit-tokens = { $count ->
    [one] token
   *[other] tokens
}
state-thinking = Claude thinks
state-writing = Claude writes
state-reading = Claude reads { $value }
state-searching = Claude searches { $value }
state-editing = Claude edits { $value }
state-creating = Claude creates { $value }
state-running = Claude runs { $value }
state-browsing = Claude browses { $value }
state-using = Claude uses { $value }
state-waiting-for-agents = { $count ->
    [one] Claude waits for 1 agent
   *[other] Claude waits for { $count } agents
}
percent = { $value }%
state-paused = Paused
state-queued = 1 prompt waiting
note-placeholder = a note for Claude, then enter · tab resumes
queued-after-the-reply = sent after the reply
agents-at-work = { $count ->
    [one] 1 agent at work
   *[other] { $count } agents in parallel
}
agents-out-of-sight = { $count ->
    [one] 1 agent
   *[other] { $count } agents
}
agent-started-by-claude = started by Claude · { $value }
agent-view-placeholder = an agent at work reads nothing typed here · escape goes back
state-agent-working = { $value } · working
state-agent-done = { $value } · done
agent-read = { $count ->
    [one] 1 file read
   *[other] { $count } files read
}
agent-searched = { $count ->
    [one] 1 search
   *[other] { $count } searches
}
agent-edited = { $count ->
    [one] 1 file changed
   *[other] { $count } files changed
}
agent-created = { $count ->
    [one] 1 file created
   *[other] { $count } files created
}
agent-ran = { $count ->
    [one] 1 command
   *[other] { $count } commands
}
agent-browsed = { $count ->
    [one] 1 page read
   *[other] { $count } pages read
}
agent-delegated = { $count ->
    [one] 1 agent started
   *[other] { $count } agents started
}
agent-used = { $count ->
    [one] 1 tool
   *[other] { $count } tools
}
session-name = session { $value }
cost-dollars = ${ $value }
quote-open = “
quote-close = ”
image-unsupported = This is not an image Claude reads: PNG, JPEG, GIF or WebP.
image-too-large = This image is over 5 MB, Claude’s limit.
image-too-many = One message carries 20 images at most, 20 MB in all.

## Conversation, when a reply does not end as Claude finished it

reply-stopped = Reply stopped.
reply-unauthorized = Anthropic rejected the key or the account.
reply-rejected = Anthropic rejected the request. If the conversation is too long, { -command-key }+N starts a new one.
reply-rate-limited = Usage limit reached. Try again later.
reply-overloaded = Claude is overloaded. Try again in a moment.
reply-unreachable = Anthropic is not answering. Check your connection, then try again.
reply-declined = Claude declined to answer this request.
reply-claude-code-failed = Claude Code could not answer.
reply-oversized = Part of the reply is over 4 MB, more than { -app-name } reads: the reply is cut and none of it is kept.
reply-crashed = { -app-name } crashed during this reply: the conversation starts afresh.
conversation-restarted = New conversation: Claude no longer sees what came before.
conversation-compacted = Conversation summarized for Claude:

## Conversation, the tools Claude uses: what each one does, in front of what it works on

tool-read = read
tool-write = write
tool-edit = edit
tool-run = $
tool-find-files = find
tool-search-text = search
tool-fetch = fetch
tool-search-web = web
tool-delegate = delegate
tool-other = tool

## Conversation, how a tool ended, after what it worked on

tool-failed = failed
tool-refused = refused
tool-stopped = stopped
tool-lines = { $count ->
    [one] line
   *[other] lines
}
tool-files = { $count ->
    [one] file
   *[other] files
}
tool-matches = { $count ->
    [one] match
   *[other] matches
}
tool-exit = exit
gutter-lines = ln
code-new = new
code-fold = fold
editor-by-claude = Claude:
editor-created-by-claude = created by Claude ·
editor-by-you = you:
editor-to-claude = Claude’s version
editor-to-before = before Claude
editor-to-yours = your changes
editor-back = back
editor-saved = saved at each keystroke
editor-not-saved = not saved
editor-outside = This file is outside the session’s folder: Dot does not open it.
editor-not-a-file = This is not a file.
editor-too-large = This file is larger than 1 MiB: too large to touch up here.
editor-not-text = This file is not text.
editor-failed = This file does not open.
editor-lost = Dot could not save { $value }: your last touch-ups are not in it.

## Prompter, when Claude asks before it goes ahead

permission-run = Run this command?
permission-change = Change this file?
permission-change-outside = Change this file, outside the project?
permission-read-outside = Read this, outside the project?
permission-fetch = Open this page?
permission-search-web = Search the web?
permission-use = Use this tool?
permission-allow = return allow
permission-deny = esc refuse
grant-command = tab always this command
grant-changes = tab every file of the project
grant-file = tab always this one
grant-host = tab always this site
grant-search-web = tab always the web
grant-tool = tab always this tool

## Settings
settings-title = Settings
settings-saved = saved on every change
settings-unsaved = The settings could not be saved.
settings-choose = choose
settings-change = change
settings-reset = default
settings-close = close
settings-default = Default
settings-rank = Setting
settings-conversation = Conversation
settings-display = Display
setting-model = Model
setting-model-hint = The model that answers in this session, from the next prompt on; a new session starts with the same.
setting-effort = Effort
setting-effort-hint = How much Claude thinks in this session before it answers. Higher: slower, dearer.
setting-summaries = Automatic summary
setting-summaries-hint = Dot summarizes the conversation when reading it again costs more than a summary.
summaries-when-they-pay = when it pays
summaries-left-to-claude = context full
setting-permissions = Claude's actions
setting-permissions-hint = Reading, writing, running a command: without a question, or a question each time.
permissions-bypass = allowed at once
permissions-ask = asked
setting-color = Colour
setting-color-hint = The colour of this session: what changes right now, its card, the tools and the gauge. Each new session takes a colour no other has.
hue-orange = orange
hue-red = red
hue-amber = amber
hue-green = green
hue-cyan = cyan
hue-blue = blue
hue-violet = violet
hue-pink = pink
setting-language = Language
setting-language-hint = The language of every text in Dot. Claude answers in the language of your question.
setting-column = Column width
setting-column-hint = The most characters on a line. A narrow window holds fewer.
column-characters = { $value } col
setting-reading-line = Reading line
setting-reading-line-hint = The height of the band where you write. It moves at once.
reading-line-middle = middle
reading-line-third = a third down
setting-caret = Caret _
setting-caret-hint = The _ of the writing line and of the reply.
caret-blinking = blinks
caret-steady = steady
setting-sessions-column = Sessions column
setting-sessions-column-hint = auto: shown as soon as the window is wide enough.
sessions-column-auto = auto
sessions-column-never = never
setting-state-colors = State colours
setting-state-colors-hint = ✓ in green, ✗ in red, stopped in amber; the context gauge in the session's colour as it rises, and from 80 %.
setting-tool-colors = Tool colours
setting-tool-colors-hint = The glyph of each tool (▤ ⌕ ✎ ⌘ ⧉), in the session's colour.
toggle-on = yes
toggle-off = no
setting-size = Size
setting-size-hint = Size of everything Dot draws: text, margins, status bar, settings; 80 % was the former size.
size-percent = { $value } %
settings-outputs = Command outputs
setting-output-cleaning = Terminal cleanup
setting-output-cleaning-hint = Colours and lines redrawn in place (progress bars) removed from what Claude reads; a redrawn line is told once.
setting-output-rewriting = Lossless rewriting
setting-output-rewriting-hint = Repeated paths written once, file lists folded, JSON without spaces, trees, tables, shorter git status and git log; nothing is lost.
setting-output-progress = Progress counted
setting-output-progress-hint = Progress of known tools (builds, installs) and passed tests counted: [+12 lines: build progress]; errors stay whole.
setting-contents-cut = File and diff cut
setting-contents-cut-hint = A file or a diff longer than the first size is brought down toward the second, kept whole in /tmp/dot-whole; measured: 10 → 6 kB.
setting-outputs-cut = Other output cut
setting-outputs-cut-hint = Any other output longer than the first size is brought down toward the second when the cut removes 4 kB or more, kept whole in /tmp/dot-whole; measured: 7 → 3 kB.
cut-never = never
cut-bounds = { $value } kB
settings-connection = Connection
setting-folder = Folder
setting-folder-hint = enter opens the folder picker; a new session opens in the one chosen.
folder-none = none
folder-picker-prompt = Open a session here
setting-account = Account
setting-account-hint = enter changes the connection: Claude plan, Anthropic account or API key.
account-none = none
setting-api-key = API key
setting-api-key-hint = enter to paste one, enter again to check it. It goes to the keychain, never in plain text.
api-key-none = none
api-key-kept = in the keychain
api-key-checking = checking…
setting-sign-out = Sign out
setting-sign-out-hint = enter, then enter again to confirm. Dot forgets its connection; Claude Code and ant stay signed in.
sign-out-confirm = enter again to confirm
selection-label = Selection
selection-characters = { $value } chars
selection-lines = { $value } lines
selection-copy = copy
selection-copied = copied
