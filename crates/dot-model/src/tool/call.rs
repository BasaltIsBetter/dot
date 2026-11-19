use dot_api::{LineRange, SearchShow, TextSearch, ToolCall};
use serde::Deserialize;
use serde_json::Value;

const TO_THE_END: i64 = -1;
const UNKNOWN_TOOL: &str = "Dot has no tool by this name.";
const NO_COMMAND: &str = "Give the bash tool a command to run, or restart: true.";
const UNDO_UNSUPPORTED: &str =
    "Dot does not undo edits: view the file and change it back with str_replace instead.";
const BAD_RANGE: &str =
    "view_range takes two line numbers counted from 1: [first, last], or [first, -1] to the end.";

#[derive(Deserialize)]
struct Bash {
    command: Option<String>,
    #[serde(default)]
    restart: bool,
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Editor {
    View {
        path: String,
        view_range: Option<[i64; 2]>,
    },
    Create {
        path: String,
        file_text: String,
    },
    StrReplace {
        path: String,
        old_str: String,
        #[serde(default)]
        new_str: String,
    },
    Insert {
        path: String,
        insert_line: u64,
        #[serde(alias = "new_str")]
        insert_text: String,
    },
    UndoEdit,
}

#[derive(Deserialize)]
struct FindFiles {
    pattern: String,
    path: Option<String>,
}

#[derive(Deserialize)]
struct SearchText {
    pattern: String,
    path: Option<String>,
    glob: Option<String>,
    #[serde(default)]
    ignore_case: bool,
    #[serde(default)]
    output: Output,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Output {
    #[default]
    Files,
    Lines,
    Counts,
}

pub(crate) fn call_of(name: &str, input: &Value) -> Result<ToolCall, String> {
    match name {
        "bash" => bash(read(input)?),
        "str_replace_based_edit_tool" => editor(read(input)?),
        "find_files" => {
            let FindFiles { pattern, path } = read(input)?;
            Ok(ToolCall::FindFiles {
                pattern,
                folder: path,
            })
        }
        "search_text" => Ok(search(read(input)?)),
        _ => Err(UNKNOWN_TOOL.to_owned()),
    }
}

fn read<'de, T: Deserialize<'de>>(input: &'de Value) -> Result<T, String> {
    T::deserialize(input).map_err(|error| format!("The tool input is not valid: {error}."))
}

fn bash(Bash { command, restart }: Bash) -> Result<ToolCall, String> {
    if restart {
        return Ok(ToolCall::RestartShell);
    }
    command
        .map(|command| ToolCall::Run { command })
        .ok_or_else(|| NO_COMMAND.to_owned())
}

fn editor(editor: Editor) -> Result<ToolCall, String> {
    Ok(match editor {
        Editor::View { path, view_range } => ToolCall::View {
            path,
            lines: view_range.map(range).transpose()?,
        },
        Editor::Create { path, file_text } => ToolCall::Create {
            path,
            text: file_text,
        },
        Editor::StrReplace {
            path,
            old_str,
            new_str,
        } => ToolCall::Replace {
            path,
            old: old_str,
            new: new_str,
        },
        Editor::Insert {
            path,
            insert_line,
            insert_text,
        } => ToolCall::Insert {
            path,
            after_line: insert_line,
            text: insert_text,
        },
        Editor::UndoEdit => return Err(UNDO_UNSUPPORTED.to_owned()),
    })
}

fn range([first, last]: [i64; 2]) -> Result<LineRange, String> {
    let first = u64::try_from(first)
        .ok()
        .filter(|&first| first >= 1)
        .ok_or_else(|| BAD_RANGE.to_owned())?;
    let last = match last {
        TO_THE_END => None,
        last => Some(
            u64::try_from(last)
                .ok()
                .filter(|&last| last >= first)
                .ok_or_else(|| BAD_RANGE.to_owned())?,
        ),
    };
    Ok(LineRange { first, last })
}

fn search(search: SearchText) -> ToolCall {
    ToolCall::SearchText(TextSearch {
        pattern: search.pattern,
        folder: search.path,
        files: search.glob,
        ignore_case: search.ignore_case,
        show: match search.output {
            Output::Files => SearchShow::Files,
            Output::Lines => SearchShow::Lines,
            Output::Counts => SearchShow::Counts,
        },
    })
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/tool/call.rs"]
mod tests;
