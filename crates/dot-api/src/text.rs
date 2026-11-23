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
    /// Status bar: label in front of the size of the conversation's base, what Claude reads
    /// before anything is said.
    StatusBase = "status-base",
    /// Status bar: label of the Claude plan's usage limit over 5 hours.
    StatusFiveHourLimit = "status-five-hour-limit",
    /// Status bar: label of the Claude plan's usage limit over 7 days.
    StatusWeeklyLimit = "status-weekly-limit",
    /// Prompter: suggestion shown in the empty field, asks Claude to explain the project.
    SuggestionExplainProject = "suggestion-explain-project",
    /// Prompter: suggestion shown in the empty field, asks Claude to find and fix a bug.
    SuggestionFixBug = "suggestion-fix-bug",
    /// Prompter: suggestion shown in the empty field, asks Claude to write the missing tests.
    SuggestionWriteTests = "suggestion-write-tests",
    /// Prompter: suggestion shown in the empty field, asks Claude to review the latest changes.
    SuggestionReviewChanges = "suggestion-review-changes",
    /// Prompter: suggestion shown in the empty field, asks Claude to simplify the code.
    SuggestionSimplifyCode = "suggestion-simplify-code",
    /// Prompter: suggestion shown in the empty field, asks Claude where startup is handled.
    SuggestionFindStartup = "suggestion-find-startup",
    /// Prompter: suggestion shown in the empty field, asks Claude to summarize recent commits.
    SuggestionSummarizeCommits = "suggestion-summarize-commits",
    /// Prompter: suggestion shown in the empty field, asks Claude to run the tests and fix them.
    SuggestionRunTests = "suggestion-run-tests",
    /// Key help, shown beside the reading line at rest: the key that sends the prompt.
    HelpSendKey = "help-send-key",
    /// Key help: what that key does, send the prompt.
    HelpSend = "help-send",
    /// Key help: the key that goes to a new line in the prompt.
    HelpNewLineKey = "help-new-line-key",
    /// Key help: what that key does, go to a new line.
    HelpNewLine = "help-new-line",
    /// Key help: the key that pauses Claude's reply.
    HelpPauseKey = "help-pause-key",
    /// Key help: what that key does, pause the reply to slip a note in.
    HelpPause = "help-pause",
    /// Key help: the key that stops Claude's reply.
    HelpStopKey = "help-stop-key",
    /// Key help: what that key does, stop the reply.
    HelpStop = "help-stop",
    /// Key help: what scrolls back through the conversation.
    HelpReviewKey = "help-review-key",
    /// Key help: what scrolling does, read the conversation again.
    HelpReview = "help-review",
    /// Key help: the keys that switch to the session above or below.
    HelpSwitchKey = "help-switch-key",
    /// Key help: what those keys do, switch session.
    HelpSwitch = "help-switch",
    /// Prompter and conversation: label in front of an attached image.
    ImageLabel = "image-label",
    /// Prompter and conversation: name of an image pasted rather than dropped as a file.
    ImagePasted = "image-pasted",
    /// Size unit of an image under a mebibyte.
    SizeKilobytes = "size-kilobytes",
    /// Size unit of an image from a mebibyte.
    SizeMegabytes = "size-megabytes",
    /// Mark between the whole part and the tenths of a number, such as the comma of `0,8`.
    DecimalSeparator = "decimal-separator",
    /// Unit after a duration in seconds, such as `0,8 s`.
    UnitSeconds = "unit-seconds",
    /// Label of the tokens Claude wrote, beside the reading line.
    InstrumentTokens = "instrument-tokens",
    /// Label of how long a reply took, beside the reading line.
    InstrumentTime = "instrument-time",
    /// Unit after a rate of tokens per second.
    UnitTokensPerSecond = "unit-tokens-per-second",
    /// Word after a number of tokens, in the plural form the count asks for.
    UnitTokens = "unit-tokens",
    /// What Claude does while nothing shows yet, in the status bar.
    StateThinking = "state-thinking",
    /// What Claude does while its reply arrives, in the status bar.
    StateWriting = "state-writing",
    /// Claude reads a file: the file is the value.
    StateReading = "state-reading",
    /// Claude looks for files or text: what it looks for is the value.
    StateSearching = "state-searching",
    /// Claude changes a file: the file is the value.
    StateEditing = "state-editing",
    /// Claude creates a file: the file is the value.
    StateCreating = "state-creating",
    /// Claude runs a command: the command is the value.
    StateRunning = "state-running",
    /// Claude reaches the web: the address or the search is the value.
    StateBrowsing = "state-browsing",
    /// Claude uses another tool: the tool is the value.
    StateUsing = "state-using",
    /// Claude waits for the agents it started, counted.
    StateWaitingForAgents = "state-waiting-for-agents",
    /// An amount of dollars: the amount, with its decimal separator, is the value.
    CostDollars = "cost-dollars",
    /// Name of a session opened in the window: its number is the value.
    SessionName = "session-name",
    /// What the status bar says while Claude's reply is paused.
    StatePaused = "state-paused",
    /// What the status bar adds while a prompt waits for the reply to end.
    StateQueued = "state-queued",
    /// What the empty note says while the reply is paused.
    NotePlaceholder = "note-placeholder",
    /// What follows a prompt that waits for the reply to end.
    QueuedAfterTheReply = "queued-after-the-reply",
    /// Heads the agents at work under the reply, counted.
    AgentsAtWork = "agents-at-work",
    /// Says who started an agent and what for: its task is the value.
    AgentStartedByClaude = "agent-started-by-claude",
    /// What the line of an agent's view says: nothing typed reaches an agent at work.
    AgentViewPlaceholder = "agent-view-placeholder",
    /// What the status bar says while the view of an agent at work is open: its kind is the value.
    StateAgentWorking = "state-agent-working",
    /// What the status bar says while the view of an agent that finished is open: its kind is
    /// the value.
    StateAgentDone = "state-agent-done",
    /// Files an agent read, counted.
    AgentRead = "agent-read",
    /// Searches an agent made, counted.
    AgentSearched = "agent-searched",
    /// Files an agent changed, counted.
    AgentEdited = "agent-edited",
    /// Files an agent created, counted.
    AgentCreated = "agent-created",
    /// Commands an agent ran, counted.
    AgentRan = "agent-ran",
    /// Pages an agent read on the web, counted.
    AgentBrowsed = "agent-browsed",
    /// Agents an agent started, counted.
    AgentDelegated = "agent-delegated",
    /// Other tools an agent used, counted.
    AgentUsed = "agent-used",
    /// A share in percent: the number is the value.
    Percent = "percent",
    /// Opens a quoted search pattern in the gutter of tools.
    QuoteOpen = "quote-open",
    /// Closes a quoted search pattern in the gutter of tools.
    QuoteClose = "quote-close",
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
    /// Notice in the reply, in front of two token counts: the conversation was summarized, from
    /// the first size to the second.
    ConversationCompacted = "conversation-compacted",
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
    /// Gutter of tools: short unit after a count of lines read, such as `212 l.`.
    GutterLines = "gutter-lines",
    /// Code: before the count of lines of a file Claude just created, such as `new · 28 lines`.
    CodeNew = "code-new",
    /// Code: the row that folds the lines of a created file back.
    CodeFold = "code-fold",
    /// Editor: before the count of lines Claude changed in the file, such as `Claude: 3 lines`.
    EditorByClaude = "editor-by-claude",
    /// Editor: in place of [`Text::EditorByClaude`] for a file Claude created.
    EditorCreatedByClaude = "editor-created-by-claude",
    /// Editor: before the count of lines the user changed, such as `you: 1 line`.
    EditorByYou = "editor-by-you",
    /// Editor: where the backward and forward arrows lead: Claude's version of the file.
    EditorToClaude = "editor-to-claude",
    /// Editor: where the backward arrow leads: the file before Claude changed it.
    EditorToBefore = "editor-to-before",
    /// Editor: where the forward arrow leads: the user's own changes.
    EditorToYours = "editor-to-yours",
    /// Editor: what the escape key does, after its name.
    EditorBack = "editor-back",
    /// Status bar: after the file's name while the editor is open.
    EditorSaved = "editor-saved",
    /// Status bar: after the file's name when the last keystroke could not be saved.
    EditorNotSaved = "editor-not-saved",
    /// Editor: the file leads out of the session's folder.
    EditorOutside = "editor-outside",
    /// Editor: the path names a folder, a pipe, a socket or a device, not a file.
    EditorNotAFile = "editor-not-a-file",
    /// Editor: the file is larger than a person edits by hand.
    EditorTooLarge = "editor-too-large",
    /// Editor: the file is not text.
    EditorNotText = "editor-not-text",
    /// Editor: the system would not open or save the file.
    EditorFailed = "editor-failed",
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

    /// Returns the text with `value` written where the translation places it, such as the file
    /// in « Claude reads layout.rs ».
    ///
    /// By default, the text then a space and the value: a translator that knows where the value
    /// goes says so.
    fn with(&self, text: Text, value: &str) -> Cow<'_, str> {
        Cow::Owned(format!("{} {value}", self.translate(text)))
    }
}

#[cfg(test)]
#[path = "../../../tests/dot-api/text.rs"]
mod tests;
