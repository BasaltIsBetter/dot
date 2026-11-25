FROM node:24-bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl git jq procps ripgrep \
    && apt-get clean
RUN npm install --global @anthropic-ai/claude-code

# The container runs as whoever runs Dot: any user must be at home here.
RUN mkdir -p /home/agent && chmod 0777 /home/agent
ENV HOME=/home/agent DISABLE_AUTOUPDATER=1
WORKDIR /home/agent
