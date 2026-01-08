# Open Agent – Project Guide

Open Agent is a managed control plane for OpenCode-based agents. The backend **does not** run model inference or autonomous logic; it delegates execution to an OpenCode server and focuses on orchestration, telemetry, and workspace/library management.

## Architecture Summary

- **Backend (Rust/Axum)**: mission orchestration, workspace/chroot management, MCP registry, Library sync.
- **OpenCode Client**: `src/opencode/` and `src/agents/opencode.rs` (thin wrapper).
- **Dashboards**: `dashboard/` (Next.js) and `ios_dashboard/` (SwiftUI).

## Core Concepts

- **Library**: Git-backed config repo (skills, commands, agents, MCPs). `src/library/`.
- **Workspaces**: Host or chroot environments with their own skills and plugins. `src/workspace.rs` manages workspace lifecycle and syncs skills to `.opencode/skill/`.
- **Missions**: Agent selection + workspace + conversation. Execution is delegated to OpenCode and streamed to the UI.

## Scoping Model

- **Global**: Auth, providers, MCPs (run on HOST machine), agents, commands
- **Per-Workspace**: Skills, plugins/hooks, installed software (chroot only), file isolation
- **Per-Mission**: Agent selection, workspace selection, conversation history

MCPs are global because they run as child processes on the host, not inside chroots.
Skills and plugins are synced to workspace `.opencode/` directories.

## Design Guardrails

- Do **not** reintroduce autonomous agent logic (budgeting, task splitting, verification, model selection). OpenCode handles execution.
- Keep the backend a thin orchestrator: **Start Mission → Stream Events → Store Logs**.
- Avoid embedding provider-specific logic in the backend. Provider auth is managed via OpenCode config + dashboard flows.

## Common Entry Points

- `src/api/routes.rs` – API routing and server startup.
- `src/api/control.rs` – mission control session, SSE streaming.
- `src/api/mission_runner.rs` – per-mission execution loop.
- `src/workspace.rs` – workspace lifecycle + OpenCode config generation.
- `src/opencode/` – OpenCode HTTP + SSE client.

## Testing

Testing of the backend cannot be done locally as it requires Linux-specific tools (desktop MCP). Deploy as root on `95.216.112.253` (use local SSH key `cursor`). Always prefer debug builds for speed.

Fast deploy loop (sync source only, build on host):

```bash
# from macOS
rsync -az --delete \
  --exclude target --exclude .git --exclude dashboard/node_modules \
  /Users/thomas/conductor/workspaces/open_agent/vaduz-v1/ \
  root@95.216.112.253:/opt/open_agent/vaduz-v1/

# on host
cd /opt/open_agent/vaduz-v1
cargo build --bin open_agent
# restart services when needed:
# - OpenCode server: `opencode.service`
# - Open Agent backend: `open_agent.service`
```

Notes to avoid common deploy pitfalls:
- Always include the SSH key in rsync: `-e "ssh -i ~/.ssh/cursor"` (otherwise auth will fail in non-interactive shells).
- The host uses rustup; build with `source /root/.cargo/env` so the newer toolchain is on PATH.

## Notes

- OpenCode config files are generated per workspace; do not keep static `opencode.json` in the repo.
- Chroot workspaces require root and Ubuntu/Debian tooling.
