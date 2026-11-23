use super::*;

fn told(tool: &str, lines: &[&str]) -> Vec<Option<&'static str>> {
    (0..lines.len())
        .map(|at| {
            RULES
                .iter()
                .filter(|rule| rule.tools.contains(&tool))
                .find(|rule| rule.drops(lines, at))
                .map(|rule| rule.what)
        })
        .collect()
}

#[test]
fn what_terraform_waits_for_and_sets_up() {
    let refresh = Some(STATE_REFRESHES);
    let install = Some(INSTALL_PROGRESS);
    let lines = [
        "aws_instance.web: Refreshing state... [id=i-123]",
        "data.aws_ami.ubuntu: Reading...",
        "aws_instance.web: Still creating... [10s elapsed]",
        "aws_instance.web: Creating...",
        "aws_instance.web: Creation complete after 32s [id=i-123]",
        "Initializing the backend...",
        "- Finding hashicorp/aws versions matching \"~> 5.0\"...",
        "- Installing hashicorp/aws v5.0.0...",
        "- Downloading registry.terraform.io/terraform-aws-modules/vpc/aws 5.8.1 for vpc...",
        "Acquiring state lock. This may take a few moments...",
        "Terraform has been successfully initialized!",
        "Plan: 1 to add, 0 to change, 0 to destroy.",
        "Error: Reference to undeclared resource",
    ];

    for tool in ["terraform", "tofu", "terragrunt"] {
        assert_eq!(
            told(tool, &lines),
            [
                refresh, refresh, refresh, None, None, install, install, install, install, install,
                None, None, None
            ],
            "{tool}"
        );
    }
}

#[test]
fn what_pulumi_links_to_and_the_resources_it_is_working_on() {
    let linked = Some(LINKS);
    let working = Some(RESOURCES_IN_PROGRESS);
    let lines = [
        "View in Browser (Ctrl+O): https://app.pulumi.com/me/app/dev/updates/12",
        "Permalink: https://app.pulumi.com/me/app/dev/updates/12",
        "Duration: 12s",
        " +   aws:s3:Bucket site creating (0s)",
        " ~   aws:s3:Bucket site updating (2s)",
        "     pulumi:pulumi:Stack app-dev refreshing",
        "Resources:",
        "    + 1 created",
        "error: update failed",
        "creating (a sentence)",
    ];

    assert_eq!(
        told("pulumi", &lines),
        [
            linked, linked, linked, working, working, working, None, None, None, None
        ]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told(
            "pulumi",
            &["aws_instance.web: Refreshing state... [id=i-123]"]
        ),
        [None]
    );
    assert_eq!(
        told("terraform", &["Permalink: https://app.pulumi.com"]),
        [None]
    );
}
