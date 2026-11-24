# RTK in Dot

Part of the rules of `dot-compact` (`src/report/rules/`) is adapted from RTK, by rtk-ai and rtk-ai Labs, <https://github.com/rtk-ai/rtk>, version 0.49.0, under the Apache License 2.0. Its license is `LICENSE` in this folder, word for word. RTK ships no `NOTICE` file.

## What Dot takes from RTK

No source file of RTK is copied. Dot rewrote in Rust, as rules of its own:

- the line patterns of RTK's declarative filters (`src/filters/*.toml`) and of its Gradle and Maven filters;
- the start-up lines of a GitLab runner, the lines of `mix compile` and the steps of `next build`;
- three ideas: a `psql` table without its padding, the folder `wc` lines share, the restacked branches of `gt`.

## What Dot changed

- Each line Dot removes is counted in square brackets where it was, such as `[12 lines: build progress]`; RTK drops lines without a count.
- A rule only applies when the command names its tool.

## What Dot leaves out

- RTK's limits on how many lines, or how long a line, it keeps: a cut without a count. That is all its filters of `ps`, `df`, `du`, `iptables`, `gcloud`, `systemctl`, `hadolint`, `yamllint` and `fail2ban-client` do.
- Removing the comments of the files it reads, rewriting the commands, removing the addresses in `gt log`.
