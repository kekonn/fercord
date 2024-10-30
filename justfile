tunnel_port := '8888'

[group('tools')]
tunnel port=tunnel_port:
    tailscale funnel {{port}}