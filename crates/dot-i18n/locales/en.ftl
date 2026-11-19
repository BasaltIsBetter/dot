### Dot texts in English.
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
error-window = macOS refused to open the { -app-name } window.
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
sign-in-key-hint = cmd+V paste · return connect · esc back
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
sign-in-storage-failed = The macOS keychain refused access.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5h
status-weekly-limit = 7d
status-effort = effort
effort-low = low
effort-medium = medium
effort-high = high
effort-extra-high = extra high
effort-max = max

## Prompter, above the status bar

prompter-placeholder = Write to Claude · return send · shift+return new line
prompter-replying = Claude is replying · esc stop

## Images, in the prompter and the conversation

image-label = image
image-pasted = pasted image
size-kilobytes = KB
size-megabytes = MB
image-unsupported = This is not an image Claude reads: PNG, JPEG, GIF or WebP.
image-too-large = This image is over 5 MB, Claude’s limit.

## Conversation, when a reply does not end as Claude finished it

reply-stopped = Reply stopped.
reply-unauthorized = Anthropic rejected the key or the account.
reply-rejected = Anthropic rejected the request.
reply-rate-limited = Usage limit reached. Try again later.
reply-overloaded = Claude is overloaded. Try again in a moment.
reply-unreachable = Anthropic is not answering. Check your connection, then try again.
reply-declined = Claude declined to answer this request.
reply-claude-code-failed = Claude Code could not answer.

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
