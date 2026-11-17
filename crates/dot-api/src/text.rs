use std::borrow::Cow;

macro_rules! texts {
    ($($(#[doc = $doc:literal])* $variant:ident = $id:literal,)+) => {
        /// Text shown to the user, named by its identifier in the translation files.
        ///
        /// The text itself is never written in the code: a [`Translator`] reads it in the user's
        /// language.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Text {
            $($(#[doc = $doc])* $variant,)+
        }

        impl Text {
            /// Every text, in declaration order.
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            /// Returns the text's identifier in the translation files (`.ftl`).
            #[must_use]
            pub const fn id(self) -> &'static str {
                match self {
                    $(Self::$variant => $id,)+
                }
            }
        }
    };
}

texts! {
    /// Title of the main window.
    WindowTitle = "window-title",
    /// App menu: opens the About window.
    MenuAbout = "menu-about",
    /// App menu: macOS Services submenu.
    MenuServices = "menu-services",
    /// App menu: hides Dot.
    MenuHide = "menu-hide",
    /// App menu: hides the other apps.
    MenuHideOthers = "menu-hide-others",
    /// App menu: shows all apps again.
    MenuShowAll = "menu-show-all",
    /// App menu: quits Dot.
    MenuQuit = "menu-quit",
    /// Error: the macOS event loop did not run.
    ErrorEventLoop = "error-event-loop",
    /// Error: macOS refused to create the window.
    ErrorWindow = "error-window",
    /// Error: the window could not be attached to the GPU.
    ErrorSurface = "error-surface",
    /// Error: no compatible graphics card.
    ErrorAdapter = "error-adapter",
    /// Error: the graphics card refused to open a drawing session.
    ErrorDevice = "error-device",
    /// Error: the graphics card cannot draw into the window.
    ErrorUnsupportedSurface = "error-unsupported-surface",
    /// Error: the drawing surface was lost.
    ErrorSurfaceLost = "error-surface-lost",
    /// Error: the text could not be drawn.
    ErrorText = "error-text",
    /// Sign-in screen: title.
    SignInTitle = "sign-in-title",
    /// Sign-in screen: what the user chooses.
    SignInIntro = "sign-in-intro",
    /// Sign-in screen: the choice of a Claude account.
    SignInClaudeAccount = "sign-in-claude-account",
    /// Sign-in screen: what the Claude account choice uses.
    SignInClaudeAccountDetail = "sign-in-claude-account-detail",
    /// Sign-in screen: the choice of an API key.
    SignInApiKey = "sign-in-api-key",
    /// Sign-in screen: what the API key choice uses.
    SignInApiKeyDetail = "sign-in-api-key-detail",
    /// Sign-in screen: keys that choose and confirm.
    SignInChooseHint = "sign-in-choose-hint",
    /// Sign-in screen: asks for the API key.
    SignInKeyPrompt = "sign-in-key-prompt",
    /// Sign-in screen: keys that paste, connect and go back.
    SignInKeyHint = "sign-in-key-hint",
    /// Sign-in screen: the key is being checked with Anthropic.
    SignInCheckingKey = "sign-in-checking-key",
    /// Sign-in screen: Claude Code is signing in, in the browser.
    SignInWaitingForClaudeCode = "sign-in-waiting-for-claude-code",
    /// Sign-in screen: key that cancels.
    SignInCancelHint = "sign-in-cancel-hint",
    /// Sign-in screen: the text is not shaped like an API key.
    SignInKeyMalformed = "sign-in-key-malformed",
    /// Sign-in screen: Anthropic refused the key.
    SignInKeyRejected = "sign-in-key-rejected",
    /// Sign-in screen: Anthropic could not be reached.
    SignInAnthropicUnreachable = "sign-in-anthropic-unreachable",
    /// Sign-in screen: Claude Code is not installed.
    SignInClaudeCodeMissing = "sign-in-claude-code-missing",
    /// Sign-in screen: signing in through Claude Code did not complete.
    SignInClaudeCodeFailed = "sign-in-claude-code-failed",
    /// Sign-in screen: the macOS keychain refused access.
    SignInStorageFailed = "sign-in-storage-failed",
}

/// Translates [`Text`]s into the language chosen at startup.
pub trait Translator {
    /// Returns the text in the translator's language.
    ///
    /// A text missing from the translation files yields its identifier ([`Text::id`]), never an
    /// empty string.
    fn translate(&self, text: Text) -> Cow<'_, str>;
}

#[cfg(test)]
#[path = "../../../tests/dot-api/text.rs"]
mod tests;
