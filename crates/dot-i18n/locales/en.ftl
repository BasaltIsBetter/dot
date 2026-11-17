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
sign-in-api-key = API key
sign-in-api-key-detail = billed per use, Anthropic Console
sign-in-choose-hint = ↑↓ choose · return confirm
sign-in-key-prompt = Paste your Anthropic API key.
sign-in-key-hint = cmd+V paste · return connect · esc back
sign-in-checking-key = Checking the key with Anthropic…
sign-in-waiting-for-claude-code = Signing in in your browser, through Claude Code…
sign-in-cancel-hint = esc cancel
sign-in-key-malformed = This is not an Anthropic API key: it starts with sk-ant-.
sign-in-key-rejected = Anthropic rejected this key.
sign-in-anthropic-unreachable = Anthropic is not answering. Check your connection, then try again.
sign-in-claude-code-missing = Claude Code is not installed. Install it, then try again.
sign-in-claude-code-failed = Signing in through Claude Code did not complete.
sign-in-storage-failed = The macOS keychain refused access.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5h
status-weekly-limit = 7d
