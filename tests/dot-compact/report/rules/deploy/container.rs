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

const TRANSFER: Option<&str> = Some(TRANSFER_PROGRESS);

#[test]
fn the_layers_of_an_image_being_pulled_or_pushed() {
    assert_eq!(
        told(
            "docker",
            &[
                "a1b2c3d4e5f6: Pulling fs layer",
                "a1b2c3d4e5f6: Downloading [=====>     ]  12MB/40MB",
                "a1b2c3d4e5f6: Pull complete",
                "0123456789ab: Layer already exists",
                "Digest: sha256:0123456789abcdef",
                "Status: Downloaded newer image for rust:1.100",
                "not-a-layer!: Waiting",
                "a1b2c3d4e5f6: Something else",
                "a1b2c3d4e5f: Waiting",
                "a1b2c3d4e5f67: Waiting"
            ]
        ),
        [
            TRANSFER, TRANSFER, TRANSFER, TRANSFER, None, None, None, None, None, None
        ]
    );
}

#[test]
fn what_is_copied_between_image_stores() {
    let lines = [
        "Getting image source signatures",
        "Copying blob sha256:0123456789abcdef",
        "Copying config sha256:0123456789abcdef",
        "Writing manifest to image destination",
        "Storing signatures",
        "Trying to pull docker.io/library/rust:1.100...",
        "Error: copying blob failed",
    ];

    for tool in ["skopeo", "podman", "buildah"] {
        assert_eq!(
            told(tool, &lines),
            [TRANSFER, TRANSFER, TRANSFER, TRANSFER, TRANSFER, None, None],
            "{tool}"
        );
    }
}

#[test]
fn the_steps_of_an_image_build() {
    let build = Some(IMAGE_BUILD);

    assert_eq!(
        told(
            "docker",
            &[
                "#5 [2/7] RUN apt-get update",
                "#5 0.345 Reading package lists...",
                "#5 DONE 1.2s",
                "#6 CACHED",
                "#7 sha256:abc 1.2MB / 3.4MB 0.2s",
                "#8 exporting layers",
                "#9 ERROR: process did not complete successfully",
                "# a comment",
                "#x not a step",
                "#12",
                "# 0.5 is not numbered",
                "#x 0.345 is not numbered either",
                " ---> Using cache",
                "Removing intermediate container 0123456789ab",
                "Step 3/9 : RUN make",
                "Successfully built 0123456789ab"
            ]
        ),
        [
            None, build, build, build, build, build, None, None, None, None, None, None, build,
            build, None, None
        ]
    );
    assert_eq!(told("podman", &["#5 DONE 1.2s"]), [build]);
}

#[test]
fn what_kubernetes_leaves_unchanged_and_waits_for() {
    assert_eq!(
        told(
            "kubectl",
            &[
                "deployment.apps/web unchanged",
                "service/web configured",
                "configmap/web created",
                "Waiting for deployment \"web\" rollout to finish: 1 of 3 updated replicas are available...",
                "deployment \"web\" successfully rolled out",
                "error: unable to recognize \"web.yaml\""
            ]
        ),
        [
            Some(UNCHANGED),
            None,
            None,
            Some(ROLLOUT_PROGRESS),
            None,
            None
        ]
    );
}

#[test]
fn the_warnings_of_the_kubernetes_client_are_dated() {
    let lines = [
        "W1119 18:00:00.123456   12345 warnings.go:70] policy/v1beta1 PodSecurityPolicy is deprecated",
        "Warning: resource is missing the annotation",
        "W11 short",
        "Wabcd is not a date",
        "W1119x is glued",
        "W1119",
        "Error: INSTALLATION FAILED",
    ];
    let warned = Some(CLIENT_WARNINGS);

    for tool in ["helm", "kubectl", "oc"] {
        assert_eq!(
            told(tool, &lines),
            [warned, None, None, None, None, None, None],
            "{tool}"
        );
    }
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told("kubectl", &["a1b2c3d4e5f6: Pull complete", "#5 DONE 1.2s"]),
        [None, None]
    );
    assert_eq!(told("docker", &["service/web unchanged"]), [None]);
}
