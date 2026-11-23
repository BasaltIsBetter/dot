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
fn what_git_says_while_it_transfers_and_the_modes_of_a_commit() {
    let lines = [
        "Enumerating objects: 9, done.",
        "Delta compression using up to 6 threads",
        "remote: Resolving deltas: 100% (3/3), completed with 3 local objects.",
        "Receiving objects: 100% (5/5), done.",
        "To github.com:me/dot.git",
        "   305a7ed..dbe543b  main -> main",
        " create mode 100644 src/a.rs",
        " delete mode 100644 src/old.rs",
        " rename src/{a.rs => b.rs} (100%)",
        " 3 files changed, 10 insertions(+), 2 deletions(-)",
        "[main dbe543b] add the compactor",
        "error: failed to push some refs",
    ];
    let modes = Some(FILE_MODES);

    assert_eq!(
        told("git", &lines),
        [
            TRANSFER, TRANSFER, TRANSFER, TRANSFER, None, None, modes, modes, modes, None, None,
            None
        ]
    );
    assert_eq!(
        told("gh", &lines)[..4],
        [TRANSFER, TRANSFER, TRANSFER, TRANSFER]
    );
    assert_eq!(told("gh", &lines)[6], None);
}

#[test]
fn what_ssh_says_when_asked_to_be_verbose() {
    let lines = [
        "debug1: Reading configuration data /etc/ssh/ssh_config",
        "debug2: resolving \"web1\" port 22",
        "debug3: send packet: type 5",
        "Linux web1 6.1.0-21-amd64",
        "Permission denied (publickey).",
        "a debug1: in a sentence",
    ];
    let debug = Some(CONNECTION_DEBUG);

    for tool in ["ssh", "scp", "sftp", "rsync"] {
        assert_eq!(
            told(tool, &lines),
            [debug, debug, debug, None, None, None],
            "{tool}"
        );
    }
}

#[test]
fn downloads_and_uploads() {
    assert_eq!(
        told(
            "wget",
            &[
                "Resolving example.com (example.com)... 93.184.216.34",
                "HTTP request sent, awaiting response... 200 OK",
                "     0K .......... .......... .......... 12% 1.2M 3s",
                "  1024K .......... ..........                100% 2.1M=0.8s",
                "2026-11-19 18:00:00 (1.2 MB/s) - 'file.tar.gz' saved [1234567/1234567]",
                "K 12 is not progress",
                "12 K apart",
                "12Kx"
            ]
        ),
        [
            TRANSFER, TRANSFER, TRANSFER, TRANSFER, None, None, None, None
        ]
    );
    assert_eq!(
        told(
            "aws",
            &[
                "Completed 1.2 MiB/12 MiB (1 MiB/s) with 3 file(s) remaining",
                "upload: ./a.txt to s3://bucket/a.txt",
                "Completed the migration",
                "3 file(s) remaining"
            ]
        ),
        [TRANSFER, None, None, None]
    );
    assert_eq!(
        told(
            "shopify",
            &[
                "  Uploading templates/index.json",
                "Downloading assets/app.js",
                "Theme pushed to the store",
                "Error: Uploading failed"
            ]
        ),
        [TRANSFER, TRANSFER, None, None]
    );
}

#[test]
fn the_replies_to_a_ping_are_not_its_statistics() {
    let lines = [
        "PING example.com (93.184.216.34): 56 data bytes",
        "64 bytes from 93.184.216.34: icmp_seq=0 ttl=56 time=11.6 ms",
        "Reply from 93.184.216.34: bytes=32 time=11ms TTL=56",
        "--- example.com ping statistics ---",
        "3 packets transmitted, 3 packets received, 0.0% packet loss",
        "Request timeout for icmp_seq 1",
        "about 64 bytes from the start",
    ];
    let replies = Some(PING_REPLIES);

    assert_eq!(
        told("ping", &lines),
        [None, replies, replies, None, None, None, None]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told("git", &["debug1: x", "64 bytes from 1.2.3.4: icmp_seq=0"]),
        [None, None]
    );
}

#[test]
fn the_meter_curl_draws_around_what_it_downloads() {
    let lines = [
        "  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current",
        "                                 Dload  Upload   Total   Spent    Left  Speed",
        "100    51  100    51    0     0   133k      0 --:--:-- --:--:-- --:--:-- 51000",
        "100 1024M  100 1024M    0     0  12.5M      0  0:01:21  0:01:21 --:--:-- 13.1M",
        "{\"name\": \"dot\"}",
        "100 51 100 51 0 0 133k 0 --:--:-- --:--:-- --:--:--",
        "100 51 100 51 0 0 133k 0 --:--:-- --:--:-- --:--:-- 51000 9",
        "100 51 100 51 0 0 133k 0 12 13 14 51000",
        "100 51 100 51 0 0 133k 0 a:b a:b a:b 51000",
        "a 51 100 51 0 0 133k 0 --:--:-- --:--:-- --:--:-- 51000",
        "100 51 100 51 0 0 133x 0 --:--:-- --:--:-- --:--:-- 51000",
        "1 2 3 4 5 6 7 8 9 10 11 12",
        "",
    ];

    assert_eq!(
        told("curl", &lines),
        [
            TRANSFER, TRANSFER, TRANSFER, TRANSFER, None, None, None, None, None, None, None, None,
            None
        ]
    );
    assert_eq!(told("wget", &lines), [None; 13]);
}
