mod container;
mod infra;

use crate::report::rules::{Lines, Rule, TRANSFER_PROGRESS, starts_with_any};

pub(crate) use self::container::RULES as CONTAINERS;
pub(crate) use self::infra::RULES as INFRASTRUCTURE;

const FILE_MODES: &str = "lines: file modes";
const CONNECTION_DEBUG: &str = "lines: connection debug";
const PING_REPLIES: &str = "lines: ping replies";

const GIT_TRANSFER: [&str; 16] = [
    "Checking out files:",
    "Compressing objects:",
    "Counting objects:",
    "Delta compression using ",
    "Enumerating objects:",
    "Filtering content:",
    "Receiving objects:",
    "Resolving deltas:",
    "Unpacking objects:",
    "Updating files:",
    "Writing objects:",
    "remote: Compressing objects",
    "remote: Counting objects",
    "remote: Enumerating objects",
    "remote: Resolving deltas",
    "remote: Total ",
];
const GIT_MODES: [&str; 4] = ["create mode ", "delete mode ", "mode change ", "rename "];
const SSH_DEBUG: [&str; 3] = ["debug1:", "debug2:", "debug3:"];
const WGET_STEPS: [&str; 5] = [
    "Connecting to ",
    "HTTP request sent",
    "Length: ",
    "Resolving ",
    "Saving to: ",
];
const AWS_PROGRESS: &str = "Completed ";
const AWS_REMAINING: &str = " remaining";
const PING_REPLY: &str = " bytes from ";
const WINDOWS_PING_REPLY: &str = "Reply from ";
const SHOPIFY_STEPS: [&str; 2] = ["Downloading", "Uploading"];
const CURL_HEADERS: [&str; 2] = [
    "  % Total    % Received % Xferd",
    "                                 Dload  Upload",
];
const CURL_FIELDS: usize = 12;
const CURL_TIMES: std::ops::Range<usize> = 8..11;
const CURL_TIME_BREAK: char = ':';
const CURL_UNITS: [char; 5] = ['k', 'M', 'G', 'T', 'P'];

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &["git", "gh", "yadm"],
        what: TRANSFER_PROGRESS,
        lines: Lines::Starting(&GIT_TRANSFER),
    },
    Rule {
        tools: &["git", "yadm"],
        what: FILE_MODES,
        lines: Lines::Indented(&GIT_MODES),
    },
    Rule {
        tools: &["ssh", "scp", "sftp", "rsync"],
        what: CONNECTION_DEBUG,
        lines: Lines::Starting(&SSH_DEBUG),
    },
    Rule {
        tools: &["wget"],
        what: TRANSFER_PROGRESS,
        lines: Lines::Told(wget_step),
    },
    Rule {
        tools: &["aws"],
        what: TRANSFER_PROGRESS,
        lines: Lines::Told(aws_progress),
    },
    Rule {
        tools: &["ping"],
        what: PING_REPLIES,
        lines: Lines::Told(ping_reply),
    },
    Rule {
        tools: &["shopify"],
        what: TRANSFER_PROGRESS,
        lines: Lines::Indented(&SHOPIFY_STEPS),
    },
    Rule {
        tools: &["curl"],
        what: TRANSFER_PROGRESS,
        lines: Lines::Told(curl_meter),
    },
];

fn wget_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at].trim_start();
    let kilobytes = line.trim_start_matches(|digit: char| digit.is_ascii_digit());
    let is_progress = kilobytes.len() < line.len() && kilobytes.starts_with("K ");
    is_progress || starts_with_any(line, &WGET_STEPS)
}

fn aws_progress(lines: &[&str], at: usize) -> bool {
    lines[at].starts_with(AWS_PROGRESS) && lines[at].contains(AWS_REMAINING)
}

fn ping_reply(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let counted =
        line.starts_with(|first: char| first.is_ascii_digit()) && line.contains(PING_REPLY);
    counted || line.starts_with(WINDOWS_PING_REPLY)
}

fn curl_meter(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    let mut fields = 0;
    let all_measured = line.split_whitespace().enumerate().all(|(place, field)| {
        fields = place + 1;
        if CURL_TIMES.contains(&place) {
            field.contains(CURL_TIME_BREAK) && field.chars().all(is_in_a_time)
        } else {
            field.starts_with(|first: char| first.is_ascii_digit())
                && field.chars().all(is_in_an_amount)
        }
    });
    starts_with_any(line, &CURL_HEADERS) || (all_measured && fields == CURL_FIELDS)
}

fn is_in_a_time(character: char) -> bool {
    character.is_ascii_digit() || character == CURL_TIME_BREAK || character == '-'
}

fn is_in_an_amount(character: char) -> bool {
    character.is_ascii_digit() || character == '.' || CURL_UNITS.contains(&character)
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/deploy/mod.rs"]
mod tests;
