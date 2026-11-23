use std::path::Path;

pub(crate) const TOOLS: &str = r#"[{"type":"bash_20250124","name":"bash"},{"type":"text_editor_20250728","name":"str_replace_based_edit_tool"},{"name":"find_files","description":"Lists the files whose path matches a glob pattern, newest first, skipping what .gitignore ignores. Use it instead of find or ls in bash whenever you look for files by name: it is faster and needs no approval inside the project.","input_schema":{"type":"object","properties":{"pattern":{"type":"string","description":"Glob pattern matched against paths relative to the folder searched, such as **/*.rs or src/**/mod.rs."},"path":{"type":"string","description":"Absolute path of the folder to search. The project folder when left out."}},"required":["pattern"],"additionalProperties":false}},{"name":"search_text","description":"Searches the text of files for a regular expression (Rust regex syntax), skipping binary files and what .gitignore ignores. Use it instead of grep or rg in bash whenever you look for text in the project: it is faster and needs no approval inside the project.","input_schema":{"type":"object","properties":{"pattern":{"type":"string","description":"Regular expression to look for, such as fn\\s+main."},"path":{"type":"string","description":"Absolute path of the file or folder to search. The project folder when left out."},"glob":{"type":"string","description":"Only search files whose path matches this glob pattern, such as *.rs."},"ignore_case":{"type":"boolean","description":"Ignore letter case. False when left out."},"output":{"type":"string","enum":["files","lines","counts"],"description":"files: the paths of the files that match (the default). lines: each matching line with its file and line number. counts: how many lines match in each file."}},"required":["pattern"],"additionalProperties":false}},{"type":"web_search_20260209","name":"web_search"},{"type":"web_fetch_20260209","name":"web_fetch","max_content_tokens":100000}]"#;

pub(crate) const CLAUDE_CODE_NOTE: &str = "You are working with the user in Dot, a desktop app \
for software development. Dot shows your replies as plain text in a monospace font and does not \
render Markdown. When one of your actions is refused, do not try to get around the refusal: ask \
the user what they want instead. Dot shortens what a command prints: a path \
repeated on many lines is written once above them, a list of files is folded with shell braces, \
and progress lines become a count in square brackets, such as [+12 lines: build progress]. The \
contents of files and diffs are never rewritten. A long output is cut: each cut is said in \
square brackets where it was made, and the last line then names the file that holds the whole \
output. Run your commands as usual, without asking for whole outputs in advance: most of the \
time what is left is enough. Only when you need what a cut left out, read those lines from \
that file, with sed -n or grep, rather than running the command again.";

pub(crate) const READER_AGENT: &str = r#"{"dot-reader":{"description":"Reads and searches the files of the project and reports what it found. Use it to find or understand something when you do not need the exact text of the files afterwards.","prompt":"You read and search the files of the project you work in, then report what you found: the answer first, then the paths and line numbers that support it. Quote exactly what you are asked to quote. You never change a file.","tools":["Read","Bash","Grep","Glob"],"model":"haiku"}}"#;

pub(crate) const READER_NOTE: &str = " To find or understand something in the project when you \
will not need the exact text of the files afterwards, hand the reading to the dot-reader agent \
and wait for its report: it reads on a cheaper model, and what it read does not enter this \
conversation, which every later request would otherwise read again. Read a file yourself when \
you are about to change it or need its exact text.";

const INTRODUCTION: &str = "You are Claude, working with the user in Dot, a desktop app for \
software development, on their Mac. Dot shows your replies as plain text in a monospace font and \
does not render Markdown.";

const NO_FOLDER: &str = "The user opened Dot without a project folder: work with absolute \
paths, and ask the user where their files are when you need them.";

const TOOL_GUIDE: &str = "Your tools:
- str_replace_based_edit_tool views files, images and folders, creates files and edits them. \
Give it absolute paths.
- find_files and search_text find files by name and search their text. Prefer them to shell \
commands for looking around: they are faster and need no approval inside the project.
- bash runs each command with /bin/bash in a fresh process: the working folder carries over from \
one command to the next, variables and functions do not. Commands get no input, so avoid \
anything interactive, and output beyond 30,000 characters is cut in the middle. Dot shortens \
what a command prints, and what find_files and search_text return: a path repeated \
on many lines is written once above them, a list of files is folded with shell braces, and \
progress lines become a count in square brackets, such as [+12 lines: build progress]. The \
contents of files and diffs are never rewritten. A long output is cut: each cut is said in \
square brackets where it was made, and the last line then says how to read the output whole. \
Start a command with DOT_RAW=1 to read its output whole.
- web_search and web_fetch look things up on the web.
Reading inside the project needs no approval. Writing a file, running a command, or reading \
outside the project first asks the user, who may refuse: then do not try to get around the \
refusal; ask them what they want instead.";

pub(crate) fn system_prompt(folder: Option<&Path>) -> String {
    let place = folder.map_or_else(
        || NO_FOLDER.to_owned(),
        |folder| {
            format!(
                "The user's project is the folder {}. Commands start there.",
                folder.display()
            )
        },
    );
    format!("{INTRODUCTION}\n\n{place}\n\n{TOOL_GUIDE}")
}

#[cfg(test)]
#[path = "../../../../tests/dot-model/tool/definition.rs"]
mod tests;
