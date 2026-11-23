use crate::report::rules::{Lines, Rule, TRANSFER_PROGRESS, starts_with_any};

const IMAGE_BUILD: &str = "lines: image build progress";
const UNCHANGED: &str = "lines: unchanged resources";
const ROLLOUT_PROGRESS: &str = "lines: rollout progress";
const CLIENT_WARNINGS: &str = "lines: client warnings";

const CONTAINER_TOOLS: [&str; 5] = ["buildah", "docker", "nerdctl", "podman", "skopeo"];
const KUBERNETES_TOOLS: [&str; 3] = ["helm", "kubectl", "oc"];
const LAYER_ID_LENGTH: usize = 12;
const LAYER_STATES: [&str; 12] = [
    "Already exists",
    "Download complete",
    "Downloading",
    "Extracting",
    "Layer already exists",
    "Mounted from",
    "Preparing",
    "Pull complete",
    "Pulling fs layer",
    "Pushed",
    "Verifying Checksum",
    "Waiting",
];
const IMAGE_COPIES: [&str; 5] = [
    "Copying blob ",
    "Copying config ",
    "Getting image source signatures",
    "Storing signatures",
    "Writing manifest to image destination",
];
const BUILD_STATES: [&str; 12] = [
    "CACHED",
    "DONE",
    "exporting ",
    "extracting ",
    "naming ",
    "preparing ",
    "pushing ",
    "resolve ",
    "sha256:",
    "transferring ",
    "unpacking ",
    "writing ",
];
const LEGACY_BUILD: [&str; 3] = [
    " ---> ",
    "Removing intermediate container ",
    "Sending build context ",
];
const WARNING_DATE_LENGTH: usize = 4;

pub(crate) const RULES: &[Rule] = &[
    Rule {
        tools: &CONTAINER_TOOLS,
        what: TRANSFER_PROGRESS,
        lines: Lines::Told(layer_step),
    },
    Rule {
        tools: &CONTAINER_TOOLS,
        what: TRANSFER_PROGRESS,
        lines: Lines::Starting(&IMAGE_COPIES),
    },
    Rule {
        tools: &CONTAINER_TOOLS,
        what: IMAGE_BUILD,
        lines: Lines::Told(image_build_step),
    },
    Rule {
        tools: &["kubectl", "oc"],
        what: UNCHANGED,
        lines: Lines::Ending(&[" unchanged"]),
    },
    Rule {
        tools: &["kubectl", "oc"],
        what: ROLLOUT_PROGRESS,
        lines: Lines::Starting(&["Waiting for "]),
    },
    Rule {
        tools: &KUBERNETES_TOOLS,
        what: CLIENT_WARNINGS,
        lines: Lines::Told(client_warning),
    },
];

fn layer_step(lines: &[&str], at: usize) -> bool {
    let Some((layer, state)) = lines[at].split_once(": ") else {
        return false;
    };
    let is_layer =
        layer.len() == LAYER_ID_LENGTH && layer.bytes().all(|byte| byte.is_ascii_hexdigit());
    is_layer && starts_with_any(state, &LAYER_STATES)
}

fn image_build_step(lines: &[&str], at: usize) -> bool {
    let line = lines[at];
    starts_with_any(line, &LEGACY_BUILD)
        || build_kit_step(line).is_some_and(|step| {
            starts_with_any(step, &BUILD_STATES)
                || step.starts_with(|first: char| first.is_ascii_digit())
        })
}

fn build_kit_step(line: &str) -> Option<&str> {
    let (number, step) = line.strip_prefix('#')?.split_once(' ')?;
    let numbered = !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit());
    numbered.then_some(step)
}

fn client_warning(lines: &[&str], at: usize) -> bool {
    let Some(date) = lines[at].strip_prefix('W') else {
        return false;
    };
    let (digits, rest) = date
        .split_at_checked(WARNING_DATE_LENGTH)
        .unwrap_or_default();
    digits.len() == WARNING_DATE_LENGTH
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && rest.starts_with(' ')
}

#[cfg(test)]
#[path = "../../../../../../tests/dot-compact/report/rules/deploy/container.rs"]
mod tests;
