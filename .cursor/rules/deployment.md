# Open Agent - Deployment Guide

## Production Server

| Property | Value |
|----------|-------|
| **Host** | `95.216.112.253` |
| **SSH Access** | `ssh root@95.216.112.253` |
| **Backend URL** | `https://agent-backend.thomas.md` |
| **Dashboard URL** | `https://agent.thomas.md` (Vercel deployment) |
| **Environment file** | `/etc/open_agent/open_agent.env` |
| **Binary location** | `/usr/local/bin/open_agent` |
| **Systemd service** | `open_agent` |
| **Source code** | `/root/open_agent` |

## Port Configuration

| Service | Local Port | Production URL |
|---------|-----------|----------------|
| Backend API | 3000 | https://agent-backend.thomas.md |
| Dashboard | 3001 | https://agent.thomas.md |

## Local Development

- **Backend API**: `http://127.0.0.1:3000` (Rust server via `cargo run`)
- **Dashboard**: `http://127.0.0.1:3001` (Next.js via `bun run dev`)
- **Environment files**: 
  - Backend: `.env` in project root
  - Dashboard: `dashboard/.env.local`

## Common Operations

### Check Service Status
```bash
ssh root@95.216.112.253 'systemctl status open_agent'
```

### View Logs
```bash
ssh root@95.216.112.253 'journalctl -u open_agent -f'
```

### Restart Service
```bash
ssh root@95.216.112.253 'systemctl restart open_agent'
```

### Edit Environment Variables
```bash
ssh root@95.216.112.253 'vim /etc/open_agent/open_agent.env'
# Then restart:
ssh root@95.216.112.253 'systemctl restart open_agent'
```

## Full Redeployment

To redeploy from scratch:

```bash
# 1. SSH into server
ssh root@95.216.112.253

# 2. Go to source directory
cd /root/open_agent

# 3. Pull latest changes
git pull

# 4. Build release binary
cargo build --release

# 5. Copy binary to /usr/local/bin
cp target/release/open_agent /usr/local/bin/open_agent

# 6. Restart service
systemctl restart open_agent

# 7. Check status
systemctl status open_agent
```

## SSH Key for Git Access

The VPS has a cursor SSH key at `~/.ssh/cursor` which has read access to private GitHub repositories. The git remote should be configured to use SSH:

```bash
# Check remote URL
git remote -v

# If needed, switch to SSH:
git remote set-url origin git@github.com:owner/open_agent.git
```

Make sure the SSH config uses the cursor key for github.com:

```bash
# ~/.ssh/config on VPS
Host github.com
    HostName github.com
    User git
    IdentityFile ~/.ssh/cursor
```

## Systemd Service Configuration

The service file is typically at `/etc/systemd/system/open_agent.service`:

```ini
[Unit]
Description=Open Agent
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/open_agent
EnvironmentFile=/etc/open_agent/open_agent.env
ExecStart=/usr/local/bin/open_agent
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Nginx Reverse Proxy

The backend is proxied through nginx at `agent-backend.thomas.md`. The nginx config typically includes:

```nginx
server {
    server_name agent-backend.thomas.md;
    
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    # SSL managed by certbot
}
```

## Security Notes

- The API requires authentication when `DEV_MODE=false`
- JWT tokens are used for dashboard authentication
- Keep `.env` and `open_agent.env` out of version control
- The agent has full machine access - be careful with what tasks you submit
