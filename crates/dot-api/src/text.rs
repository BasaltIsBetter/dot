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
    /// Sign-in screen: the choice of an Anthropic Console account.
    SignInAnthropicAccount = "sign-in-anthropic-account",
    /// Sign-in screen: what the Anthropic account choice uses.
    SignInAnthropicAccountDetail = "sign-in-anthropic-account-detail",
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
    /// Sign-in screen: `ant` is signing in to the Anthropic account, in the browser.
    SignInWaitingForAnt = "sign-in-waiting-for-ant",
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
    /// Sign-in screen: Anthropic's `ant` tool is not installed.
    SignInAntMissing = "sign-in-ant-missing",
    /// Sign-in screen: signing in to the Anthropic account through `ant` did not complete.
    SignInAccountFailed = "sign-in-account-failed",
    /// Nothing connects Dot to Claude any more.
    SignInNotConnected = "sign-in-not-connected",
    /// Sign-in screen: the macOS keychain refused access.
    SignInStorageFailed = "sign-in-storage-failed",
    /// Sign-in error: Dot itself failed while it connected.
    SignInCrashed = "sign-in-crashed",
    /// Status bar: label in front of the effort level.
    StatusEffort = "status-effort",
    /// Status bar: low effort.
    EffortLow = "effort-low",
    /// Status bar: medium effort.
    EffortMedium = "effort-medium",
    /// Status bar: high effort.
    EffortHigh = "effort-high",
    /// Status bar: extra high effort.
    EffortExtraHigh = "effort-extra-high",
    /// Status bar: maximum effort.
    EffortMax = "effort-max",
    /// Status bar: label of the Claude plan's usage limit over 5 hours.
    StatusFiveHourLimit = "status-five-hour-limit",
    /// Status bar: label of the Claude plan's usage limit over 7 days.
    StatusWeeklyLimit = "status-weekly-limit",
    /// Prompter: shown in the empty field, with the keys that send and add a line.
    PrompterPlaceholder = "prompter-placeholder",
    /// Prompter: shown in the empty field while Claude replies, with the key that stops it.
    PrompterReplying = "prompter-replying",
    /// Prompter and conversation: label in front of an attached image.
    ImageLabel = "image-label",
    /// Prompter and conversation: name of an image pasted rather than dropped as a file.
    ImagePasted = "image-pasted",
    /// Size unit of an image under a mebibyte.
    SizeKilobytes = "size-kilobytes",
    /// Size unit of an image from a mebibyte.
    SizeMegabytes = "size-megabytes",
    /// Conversation: a file dropped or pasted is not an image Claude reads.
    ImageUnsupported = "image-unsupported",
    /// Conversation: an image is larger than Claude accepts.
    ImageTooLarge = "image-too-large",
    /// Notice: the prompt already carries as many images, or as many bytes of images, as one
    /// prompt can.
    ImageTooMany = "image-too-many",
    /// Conversation: the user stopped the reply.
    ReplyStopped = "reply-stopped",
    /// Conversation: Anthropic refused the API key or the account.
    ReplyUnauthorized = "reply-unauthorized",
    /// Conversation: Anthropic refused the request itself.
    ReplyRejected = "reply-rejected",
    /// Conversation: a usage or rate limit is reached.
    ReplyRateLimited = "reply-rate-limited",
    /// Conversation: Claude is overloaded.
    ReplyOverloaded = "reply-overloaded",
    /// Conversation: Anthropic could not be reached.
    ReplyUnreachable = "reply-unreachable",
    /// Conversation: Claude declined to answer.
    ReplyDeclined = "reply-declined",
    /// Conversation: Claude Code could not answer.
    ReplyClaudeCodeFailed = "reply-claude-code-failed",
    /// Notice: a piece of the reply was larger than Dot reads; the reply was cut and not kept.
    ReplyOversized = "reply-oversized",
    /// Notice: Dot itself failed during the reply; the conversation starts afresh.
    ReplyCrashed = "reply-crashed",
    /// Notice: the user started a new conversation; Claude no longer sees the earlier one.
    ConversationRestarted = "conversation-restarted",
    /// Conversation: a tool that reads a file, an image or a folder.
    ToolRead = "tool-read",
    /// Conversation: a tool that creates or rewrites a file.
    ToolWrite = "tool-write",
    /// Conversation: a tool that changes part of a file.
    ToolEdit = "tool-edit",
    /// Conversation: a tool that runs a shell command.
    ToolRun = "tool-run",
    /// Conversation: a tool that looks for files by name.
    ToolFindFiles = "tool-find-files",
    /// Conversation: a tool that looks for text in files.
    ToolSearchText = "tool-search-text",
    /// Conversation: a tool that fetches a web page.
    ToolFetch = "tool-fetch",
    /// Conversation: a tool that searches the web.
    ToolSearchWeb = "tool-search-web",
    /// Conversation: a tool that hands a task to another agent.
    ToolDelegate = "tool-delegate",
    /// Conversation: any other tool, in front of its name.
    ToolOther = "tool-other",
    /// Conversation: a tool could not do what Claude asked.
    ToolFailed = "tool-failed",
    /// Conversation: the user refused to let a tool run.
    ToolRefused = "tool-refused",
    /// Conversation: the reply stopped before a tool finished.
    ToolStopped = "tool-stopped",
    /// Conversation: lines read or shown by a tool, after their count.
    ToolLines = "tool-lines",
    /// Conversation: files found by a tool, after their count.
    ToolFiles = "tool-files",
    /// Conversation: lines that match a search, after their count.
    ToolMatches = "tool-matches",
    /// Conversation: in front of a command's exit status.
    ToolExit = "tool-exit",
    /// Prompter: asks whether Claude may run a command.
    PermissionRun = "permission-run",
    /// Prompter: asks whether Claude may change a file of the project.
    PermissionChange = "permission-change",
    /// Prompter: asks whether Claude may change a file outside the project.
    PermissionChangeOutside = "permission-change-outside",
    /// Prompter: asks whether Claude may read outside the project.
    PermissionReadOutside = "permission-read-outside",
    /// Prompter: asks whether Claude may fetch a web page.
    PermissionFetch = "permission-fetch",
    /// Prompter: asks whether Claude may search the web.
    PermissionSearchWeb = "permission-search-web",
    /// Prompter: asks whether Claude may use another tool.
    PermissionUse = "permission-use",
    /// Prompter: the key that lets Claude go ahead once.
    PermissionAllow = "permission-allow",
    /// Prompter: the key that refuses.
    PermissionDeny = "permission-deny",
    /// Prompter: the key that always lets Claude run this command.
    GrantCommand = "grant-command",
    /// Prompter: the key that lets Claude change every file of the project.
    GrantChanges = "grant-changes",
    /// Prompter: the key that always lets Claude use this file outside the project.
    GrantFile = "grant-file",
    /// Prompter: the key that always lets Claude fetch pages of this site.
    GrantHost = "grant-host",
    /// Prompter: the key that always lets Claude search the web.
    GrantSearchWeb = "grant-search-web",
    /// Prompter: the key that always lets Claude use this tool.
    GrantTool = "grant-tool",
}

/// Translates [`Text`]s into the language chosen at startup.
pub trait Translator {
    /// Returns the text in the translator's language.
    ///
    /// A text missing from the translation files yields its identifier ([`Text::id`]), never an
    /// empty string.
    fn translate(&self, text: Text) -> Cow<'_, str>;

    /// Returns the text that follows a count of `count` things, in the plural form the language
    /// gives that number, such as `lines` after 3 and `line` after 1 in English.
    ///
    /// A text missing from the translation files yields its identifier, as with
    /// [`Translator::translate`].
    fn count(&self, text: Text, count: u64) -> Cow<'_, str>;
}

#[cfg(test)]
#[path = "../../../tests/dot-api/text.rs"]
mod tests;
