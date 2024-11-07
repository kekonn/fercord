tunnel_port := '8888'

[group('tools')]
tunnel port=tunnel_port:
    tailscale funnel {{port}}

[group('tools')]
podman:
    systemctl --user enable --now podman.socket