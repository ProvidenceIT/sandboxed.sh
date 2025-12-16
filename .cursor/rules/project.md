# Open Agent - Cursor Rules & Project Philosophy

## Project Overview

Open Agent is a minimal autonomous coding agent implemented in Rust. It is designed to be:
- **AI-maintainable**: Rust's strong type system and compiler provide immediate feedback
- **Self-contained**: No external dependencies beyond OpenRouter for LLM access
- **Full system access**: Has complete access to the entire machine (filesystem, terminal, network) - not containerized to a single project
- **Provable**: Code structured for future formal verification in Lean

### System-Wide Access Model

The agent has **full system access** by design:
- Can read/write any file anywhere on the machine using absolute paths
- Can execute any shell command in any directory
- Can search for files/content across the entire filesystem
- Default working directory is `/root` in production (configurable via `WORKING_DIR`)
- Paths can be absolute (e.g., `/var/log/syslog`) or relative to the working directory

### Directory Conventions

The agent uses an organized directory structure:
```
/root/
├── context/           # 📥 INPUT: User-provided files (uploaded via dashboard)
│   └── [task files]   #    The agent checks this FIRST for files to analyze
├── work/              # 🔨 WORKSPACE: Agent's working area
│   └── [task-name]/   #    Subfolder for each distinct task/analysis
│       ├── output/    #    Final outputs, reports, results
│       ├── temp/      #    Intermediate/temporary files
│       └── notes.md   #    Task notes, findings, decisions
└── tools/             # 🛠️ TOOLBOX: Reusable scripts & utilities
    ├── README.md      #    Document each tool's purpose & usage
    └── [tool-name]/   #    One folder per tool/script
```

- **`/root/context/`**: Where users upload files for the agent to analyze (dashboard file explorer defaults here)
- **`/root/work/`**: Agent creates organized subfolders here for each task
- **`/root/tools/`**: Persistent helper scripts/tools the agent creates for reuse

## Architecture (v2: Hierarchical Agent Tree)

### Agent Hierarchy
```
                    ┌─────────────┐
                    │  RootAgent  │
                    └──────┬──────┘
         ┌─────────────────┼─────────────────┐
         ▼                 ▼                 ▼
 ┌───────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐
 │ Complexity    │ │   Model     │ │    Task     │ │ Verifier │
 │ Estimator     │ │  Selector   │ │  Executor   │ │          │
 └───────────────┘ └─────────────┘ └─────────────┘ └──────────┘
```

### Agent Types

| Type | Role | Children |
|------|------|----------|
| **RootAgent** | Top-level orchestrator, receives API tasks | All leaf types |
| **NodeAgent** | Intermediate orchestrator for subtasks | Executor, Verifier |
| **ComplexityEstimator** | Estimates task difficulty (0-1 score) | None (leaf) |
| **ModelSelector** | Picks optimal model (U-curve optimization) | None (leaf) |
| **TaskExecutor** | Executes tasks using tools | None (leaf) |
| **Verifier** | Validates completion (hybrid) | None (leaf) |

### Task Flow
1. Receive task via HTTP API
2. **Estimate Complexity** (ComplexityEstimator)
3. If complex: **Split into subtasks** with budget allocation
4. **Select Model** for each (sub)task (U-curve cost optimization)
5. **Execute** using tools (TaskExecutor)
6. **Verify** completion (Verifier: programmatic → LLM fallback)
7. **On failure**: Analyze signals → smart retry (upgrade/downgrade model)
8. Aggregate results and return

### U-Curve Model Selection
```
Cost
  ^
  |    *                         *
  |     *                       *
  |        *       *         *
  |           * *     * *
  |            *       *
  +-------------------------> Model Capability
      (cheap/weak)    (optimal)    (expensive/strong)
```
- Cheap models: low per-token cost, high failure rate, more retries
- Expensive models: high per-token cost, low failure rate
- **Optimal**: minimizes expected total cost

### Smart Retry Strategy (Budget Overflow)

When task execution fails, the system analyzes **why** it failed and selects the appropriate retry strategy:

| Failure Mode | Signals | Retry Action |
|--------------|---------|--------------|
| **Model Capability Insufficient** | Repetitive actions, high tool failure rate, stuck in loops | **Upgrade** to smarter model |
| **Budget Exhausted With Progress** | High tool success rate, files modified, partial results | **Continue** same model or try **cheaper** model |
| **External Error** | API errors, network issues, rate limits | **Retry** same configuration |
| **Task Infeasible** | Consistent failures across models | **Do not retry** |

#### Execution Signals Tracked
```rust
ExecutionSignals {
    iterations: u32,           // How many LLM calls made
    successful_tool_calls: u32, // Tools that succeeded
    failed_tool_calls: u32,     // Tools that failed
    files_modified: bool,       // Any files created/changed
    repetitive_actions: bool,   // Stuck in loops
    partial_progress: bool,     // Making progress
    cost_spent_cents: u64,      // Budget used
}
```

#### Model Upgrade/Downgrade Ladder
```
┌─────────────────────────┐
│ anthropic/claude-sonnet-4.5 │  ← Top tier
├─────────────────────────┤
│ anthropic/claude-3.5-sonnet │
├─────────────────────────┤
│ anthropic/claude-haiku-4.5  │  ← Budget tier
└─────────────────────────┘
```

The `FailureAnalysis` provides:
- `mode`: Why it failed
- `confidence`: How certain (0.0-1.0)
- `evidence`: Human-readable reasons
- `recommendation`: What to do next

## Module Structure

```
src/
├── agents/                # Hierarchical agent system
│   ├── mod.rs             # Agent traits (Agent, OrchestratorAgent, LeafAgent)
│   ├── types.rs           # AgentId, AgentType, AgentResult, Complexity
│   ├── context.rs         # Shared context for agent tree
│   ├── tree.rs            # Tree structure management
│   ├── orchestrator/      # Orchestrator agents
│   │   ├── root.rs        # RootAgent (top-level)
│   │   └── node.rs        # NodeAgent (intermediate)
│   └── leaf/              # Leaf agents (specialized workers)
│       ├── complexity.rs  # ComplexityEstimator
│       ├── model_select.rs # ModelSelector with U-curve
│       ├── executor.rs    # TaskExecutor (tools in a loop)
│       └── verifier.rs    # Hybrid verification
├── task/                  # Task types with invariants
│   ├── task.rs            # Task, TaskId, TaskStatus
│   ├── subtask.rs         # Subtask, SubtaskPlan
│   └── verification.rs    # VerificationCriteria, ProgrammaticCheck
├── budget/                # Cost tracking and pricing
│   ├── budget.rs          # Budget with spend/allocate invariants
│   ├── pricing.rs         # OpenRouter pricing client
│   ├── allocation.rs      # Budget allocation strategies
│   └── retry.rs           # Smart retry strategy (failure analysis)
├── memory/                # Persistent memory & retrieval
│   ├── mod.rs             # Memory subsystem exports
│   ├── supabase.rs        # PostgREST + Storage client
│   ├── embed.rs           # OpenRouter embeddings (Qwen3 8B)
│   ├── rerank.rs          # Reranker for precision retrieval
│   ├── writer.rs          # Event recording + chunking
│   ├── retriever.rs       # Semantic search + context packing
│   └── types.rs           # DbRun, DbTask, DbEvent, DbChunk
├── api/                   # HTTP interface
├── llm/                   # LLM client (OpenRouter)
├── tools/                 # Tool implementations
└── config.rs              # Configuration
```

## Memory System

### Purpose
- **Long tasks beyond context**: persist step-by-step execution so the agent can retrieve relevant context later
- **Fast query + browsing**: structured metadata in Postgres, heavy blobs in Storage
- **Embedding + rerank**: Qwen3 Embedding 8B for vectors, Qwen reranker for precision
- **Learning from execution**: store predictions vs actuals to improve estimates over time

### Data Flow
1. Agents emit events via `EventRecorder`
2. `MemoryWriter` persists to Supabase Postgres + Storage
3. Before LLM calls, `MemoryRetriever` fetches relevant context
4. On completion, run is archived with summary embedding
5. **Task outcomes recorded for learning** (complexity, cost, tokens, success)

### Storage Strategy
- **Postgres (pgvector)**: runs, tasks (hierarchical), events (preview), chunks (embeddings), **task_outcomes**
- **Supabase Storage**: full event streams (jsonl), large artifacts

## Learning System (v3 - Implemented)

### Purpose
Enable data-driven optimization of:
- **Complexity estimation**: learn actual token usage vs predicted
- **Model selection**: learn actual success rates per model/complexity
- **Budget allocation**: learn actual costs vs estimated

### Current Implementation Status (as of Dec 2024)

| Component | Learning Integration | Status |
|-----------|---------------------|--------|
| ComplexityEstimator | Queries `get_historical_context()` to adjust token estimates | ✅ Implemented |
| ModelSelector | Queries `get_model_stats()` for actual success rates | ✅ Implemented |
| TaskExecutor | Auto-discovers `/root/tools/` and injects tool inventory | ✅ Implemented |
| routes.rs | Records task outcomes via `record_task_outcome()` | ✅ Implemented |

### Architecture
```
┌──────────────────────────────────────────────────────────────────────┐
│                    Memory-Enhanced Agent Flow                         │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌──────────┐    Query similar     ┌─────────────────────────────┐   │
│  │ New Task │ ───────────────────▶│ MemoryRetriever             │   │
│  └────┬─────┘      past tasks      │  - find_similar_tasks()     │   │
│       │                            │  - get_historical_context() │   │
│       ▼                            │  - get_model_stats()        │   │
│  ┌────────────────┐                └───────────────┬─────────────┘   │
│  │ Complexity     │◀── historical context ─────────┘                 │
│  │ Estimator      │    (avg token ratio, avg cost ratio)             │
│  │ (learning)     │                                                  │
│  └────────┬───────┘                                                  │
│           │                                                          │
│           ▼                                                          │
│  ┌────────────────┐   Query: "models at complexity ~0.6"             │
│  │ Model Selector │   Returns: actual success rates, cost ratios     │
│  │ (learning)     │                                                  │
│  └────────┬───────┘                                                  │
│           │                                                          │
│           ▼                                                          │
│  ┌────────────────┐   Discovers /root/tools/ inventory               │
│  │ TaskExecutor   │──▶ record_task_outcome() ──▶ task_outcomes      │
│  │ (tool reuse)   │                                                  │
│  └────────────────┘                                                  │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Tool Reuse System

The TaskExecutor automatically discovers reusable tools:

1. **At execution start**: Scans `/root/tools/` (or `{working_dir}/tools`)
2. **Extracts documentation**: Reads README.md files and lists scripts
3. **Injects into prompt**: Adds tool inventory to system prompt
4. **Agent guidance**: Prompts agent to check existing tools before creating new ones

This enables cross-task learning where tools created in one task can be reused in future tasks.

### Database Schema: `task_outcomes`
```sql
CREATE TABLE task_outcomes (
    id uuid PRIMARY KEY,
    run_id uuid REFERENCES runs(id),
    task_id uuid REFERENCES tasks(id),
    
    -- Predictions
    predicted_complexity float,
    predicted_tokens bigint,
    predicted_cost_cents bigint,
    selected_model text,
    
    -- Actuals
    actual_tokens bigint,
    actual_cost_cents bigint,
    success boolean,
    iterations int,
    tool_calls_count int,
    
    -- Computed ratios (actual/predicted)
    cost_error_ratio float,
    token_error_ratio float,
    
    -- Similarity search
    task_description text,
    task_embedding vector(1536)
);
```

## Dashboard Auth (v4 - Minimal JWT)

### Goals
- Keep the API **private by default** in non-dev mode.
- Keep local iteration/debugging easy (explicit dev mode bypass).
- Use a minimal, single-tenant auth model (no users/orgs/RLS yet).

### How it works
- The dashboard calls `POST /api/auth/login` with a password.
- The server verifies the password and returns a **JWT** with an `exp` claim.
- The dashboard stores the JWT + exp in **`sessionStorage`**.
- When **`DEV_MODE=false`**, all API requests (including task streaming) must include:
  - `Authorization: Bearer <jwt>`

JWT validity: **30 days** by default (configurable).

### Dev mode + debugging

To debug quickly (no auth), run with:
- `DEV_MODE=true`

In `DEV_MODE=true`:
- `/api/health` will report `auth_required=false`
- The dashboard will not prompt for a password
- The API will not require the `Authorization` header

### Required env vars (when DEV_MODE=false)
- `DASHBOARD_PASSWORD`: the dashboard password
- `JWT_SECRET`: HMAC secret used to sign/verify JWTs
- `JWT_TTL_DAYS` (optional, default 30)

### Debugging with curl
Get a token:

```bash
curl -sS -X POST http://127.0.0.1:3000/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"password":"YOUR_PASSWORD"}'
```

Use the token:

```bash
curl -sS http://127.0.0.1:3000/api/tasks \
  -H "Authorization: Bearer YOUR_JWT"
```

### Notes / limitations
- This is **not** multi-tenant; the JWT only proves “dashboard knows the password”.
- The dashboard uses a **fetch-based SSE client** (instead of `EventSource`) so it can send auth headers.

## Dashboard Console (SSH + SFTP)

The dashboard includes a **Console** page that can:
- open a **full-featured TTY** (colors, interactive programs) via WebSocket → PTY → `ssh`
- browse/upload/download files via **SFTP**

### Backend endpoints
- `GET /api/console/ws` (WebSocket)
- `GET /api/fs/list?path=...`
- `POST /api/fs/upload?path=...` (multipart form-data)
- `GET /api/fs/download?path=...`
- `POST /api/fs/mkdir`
- `POST /api/fs/rm`

### Auth nuance (WebSocket)
Browsers can't set an `Authorization` header for WebSockets, so the console uses a **WebSocket subprotocol**:
- client connects with protocols: `["openagent", "jwt.<token>"]`
- server validates the JWT from `Sec-WebSocket-Protocol` (only when `DEV_MODE=false`)

### Required env vars
Set these on the backend:
- `CONSOLE_SSH_HOST` (e.g. `95.216.112.253`)
- `CONSOLE_SSH_PORT` (default `22`)
- `CONSOLE_SSH_USER` (default `root`)
- `CONSOLE_SSH_PRIVATE_KEY_PATH` (recommended), or `CONSOLE_SSH_PRIVATE_KEY_B64`, or `CONSOLE_SSH_PRIVATE_KEY`

## Dashboard package manager (Bun)

The dashboard in `dashboard/` uses **Bun** (not npm/yarn/pnpm).

```bash
cd dashboard
bun install
PORT=3001 bun dev
```

### RPC Functions
- `get_model_stats(complexity_min, complexity_max)` - Model performance by complexity tier
- `search_similar_outcomes(embedding, threshold, limit)` - Find similar past tasks
- `get_global_learning_stats()` - Overall system metrics

### Learning Integration Points
1. **ComplexityEstimator**: Query similar tasks → adjust token estimate by `avg_token_ratio`
2. **ModelSelector**: Query model stats → use actual success rates instead of heuristics
3. **TaskExecutor**: After execution → call `record_task_outcome()` with all metrics
4. **Budget**: Use historical cost ratios to add appropriate safety margins

## Design for Provability

### Conventions for Future Lean Proofs
1. **Pre/Postconditions**: Document as `/// Precondition:` and `/// Postcondition:` comments
2. **Invariants**: Document struct invariants, enforce in constructors
3. **Algebraic Types**: Use enums with exhaustive matching, no `_` catch-all
4. **Pure Functions**: Separate pure logic from IO where possible
5. **Result Types**: Never panic, always return `Result`

### Example
```rust
/// Allocate budget for a subtask.
/// 
/// # Precondition
/// `amount <= self.remaining_cents()`
/// 
/// # Postcondition
/// `self.allocated_cents` increases by exactly `amount`
pub fn allocate(&mut self, amount: u64) -> Result<(), BudgetError>
```

## Adding a New Leaf Agent

1. Create `src/agents/leaf/your_agent.rs`
2. Implement `Agent` trait:
   - `id()`, `agent_type()`, `execute()`
3. Implement `LeafAgent` trait:
   - `capability()` → add variant to `LeafCapability` enum
4. Register in `RootAgent::new()` or relevant orchestrator
5. Document pre/postconditions for provability

## API Contract

```
POST /api/task              - Submit task (uses hierarchical agent)
GET  /api/task/{id}         - Get task status and result
GET  /api/task/{id}/stream  - Stream progress via SSE
GET  /api/health            - Health check
GET  /api/runs              - List archived runs
GET  /api/runs/{id}         - Run detail + task tree
GET  /api/runs/{id}/events  - Event timeline
GET  /api/memory/search     - Semantic search across memory
```

## Environment Variables

```
OPENROUTER_API_KEY       - Required. Your OpenRouter API key
DEFAULT_MODEL            - Optional. Default: anthropic/claude-sonnet-4.5
WORKING_DIR              - Optional. Default working directory for relative paths.
                           Defaults to /root in production, current directory in dev.
                           Agent has full system access regardless of this setting.
HOST                     - Optional. Default: 127.0.0.1
PORT                     - Optional. Default: 3000
MAX_ITERATIONS           - Optional. Default: 50
SUPABASE_URL             - Required for memory. Supabase project URL
SUPABASE_SERVICE_ROLE_KEY - Required for memory. Service role key
MEMORY_EMBED_MODEL       - Optional. Default: qwen/qwen3-embedding-8b
MEMORY_RERANK_MODEL      - Optional. Default: qwen/qwen3-reranker-8b
```

### Recommended Models
- **Default (tools)**: `anthropic/claude-sonnet-4.5` - Best coding, 1M context, $3/$15 per 1M tokens
- **Budget fallback**: `anthropic/claude-3.5-haiku` - Fast, cheap, good for simple tasks
- **Complex tasks**: `anthropic/claude-opus-4.5` - Highest capability when needed

## Deployment

### Production Server
- **Host**: `95.216.112.253`
- **SSH Access**: `ssh root@95.216.112.253` (key-based auth)
- **Backend URL**: `https://agent-backend.thomas.md` (proxied to localhost:3000)
- **Dashboard URL**: `https://agent.thomas.md` (Vercel deployment)
- **Environment files**: `/etc/open_agent/open_agent.env`
- **Service**: `systemctl status open_agent` (runs as systemd service)
- **Binary**: `/usr/local/bin/open_agent`

### Local Development
- **Backend API**: `http://127.0.0.1:3000` (Rust server via `cargo run`)
- **Dashboard**: `http://127.0.0.1:3001` (Next.js via `bun run dev`)
- **Environment files**: 
  - Backend: `.env` in project root
  - Dashboard: `dashboard/.env.local`

### Accessing Environment Variables
The cursor agent has SSH access to the production server and can:
- Read/modify env variables at `/etc/open_agent/open_agent.env`
- Restart services: `systemctl restart open_agent`
- Check logs: `journalctl -u open_agent -f`

### Port Configuration
| Service | Local Port | Production URL |
|---------|-----------|----------------|
| Backend API | 3000 | https://agent-backend.thomas.md |
| Dashboard | 3001 | https://agent.thomas.md |

## Security Considerations

This agent has **full machine access** by design. It can:
- Read/write any file on the system
- Execute any shell command in any directory
- Make network requests
- Search the entire filesystem

This is intentional - the agent is designed to be a powerful system-wide assistant, not a sandboxed tool. When deploying:
- Run on a dedicated server/VM (production runs on `95.216.112.253`)
- Never expose the API publicly without authentication
- Use the built-in JWT auth system (`DASHBOARD_PASSWORD`, `JWT_SECRET`)
- Keep `.env` out of version control
- Users upload files to `/root/context/`, agent works in `/root/work/`, tools in `/root/tools/`

## Future Work

- [ ] Formal verification in Lean (extract pure logic)
- [ ] WebSocket for bidirectional streaming
- [x] Enhanced ComplexityEstimator with historical context injection (Dec 2024)
- [x] Enhanced ModelSelector with data-driven success rates (Dec 2024)
- [x] Tool reuse system with auto-discovery (Dec 2024)
- [x] Semantic code search (embeddings-based)
- [x] Multi-model support (U-curve optimization)
- [x] Cost tracking (Budget system)
- [x] Persistent memory (Supabase + pgvector)
- [x] Learning system (task_outcomes table, historical queries)
- [x] Smart retry strategy (analyze failure mode → upgrade/downgrade model)
- [x] Task outcome recording for learning (Dec 2024)

