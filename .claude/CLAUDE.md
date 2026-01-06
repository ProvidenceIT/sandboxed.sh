# Open Agent Panel - Architecture & Context

This project is a **Managed Open Code Panel**. It is a software suite installed on a server to remotely control AI agents, manage their execution environments (workspaces), and synchronize their configurations (Library).

## Core Philosophy

1.  **Orchestration, not Execution**: The backend does not run the LLM loop itself. It delegates execution to **Open Code** (or compatible agents like Claude Code) running locally or remotely.
2.  **Environment Management**: The panel's job is to provide the *place* for the agent to work (Workspaces, Chroots) and the *tools* it needs (Host MCP).
3.  **Configuration as Code**: All agent configurations (skills, prompts, MCP servers) are synced from a Git repository (The Library).

## Tech Stack

-   **Backend**: Rust (Axum, Tokio). Acts as the API server and Host MCP.
-   **Web Dashboard**: Next.js 14+ (App Router), Bun, Tailwind.
-   **iOS App**: Swift, SwiftUI.
-   **Infrastructure**: Systemd service on Ubuntu/Debian.

## Directory Structure

-   `src/`: Rust backend source.
-   `dashboard/`: Web dashboard source.
-   `ios_dashboard/`: iOS app source.
-   `context/`: (Legacy/Reference) Previous OpenCode schemas.

## Key Components

### 1. The Backend (`src/`)
-   **Mission Runner**: Manages active agent sessions.
-   **Workspace Manager**: Creates/destroys chroot environments.
-   **Library Manager**: Syncs the `.openagent/library` repo.
-   **Host MCP**: A built-in MCP server that gives agents access to the server's filesystem and tools (within the workspace).

### 2. The Library
A standard Git repository structure:
-   `skills/`: Reusable agent capabilities (YAML/JSON).
-   `commands/`: Custom shell commands/scripts.
-   `mcp/`: MCP server configurations (config.json).

### 3. Workspaces
-   **Directory**: Simple folder isolation.
-   **Chroot**: Full filesystem isolation (debootstrap).

## Development Workflow

### Backend
```bash
# Run locally
export OPENCODE_BASE_URL="http://localhost:4096"
cargo run
```

### Dashboard
```bash
cd dashboard
bun dev
```

## Legacy Notes (Cleanup Targets)
-   `src/budget`: Complex budget/pricing logic might be simplified as we delegate to Open Code.
-   `src/llm`: Direct LLM clients (OpenRouter) are likely unnecessary if we fully delegate to Open Code.
-   `src/task`: Complex verification logic might be simplified.
