tunnel_port := '8888'

[group('tools')]
tunnel port=tunnel_port:
    tailscale funnel {{port}}

[group('tools')]
podman:
    systemctl --user enable --now podman.socket

# `just changed-files ${{ github.event.pull_request.base.sha }}`
[group('ci')]
changed-files pr_hash file='changeset.txt':
    git fetch origin {{pr_hash}}
    git diff --name-only {{pr_hash}} HEAD > {{file}}