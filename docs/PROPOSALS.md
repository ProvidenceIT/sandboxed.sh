# Open Agent Enhancement Proposals

This document contains brainstormed proposals for improving the agent system.

---

## Implementation Status

| Proposal | Status | Notes |
|----------|--------|-------|
| Progress Checkpoints | ❌ Not Started | Complex, needs iteration tracking changes |
| Parallel Missions | ✅ **Partially Implemented** | Backend ready, UI pending |
| Optimized Prompts | ✅ Implemented | See `scripts/prompts/` |
| Bug Fixes | ✅ Done | Model override, command safety, Gemini/Kimi fixes |
| Context Isolation | 🟡 **Prompt-Level Only** | v2 prompt available, backend changes pending |

### Parallel Missions - Implementation Details

**Backend (Implemented):**
- `MAX_PARALLEL_MISSIONS` config option (default: 1)
- `MissionRunner` abstraction in `src/api/mission_runner.rs`
- SSE events now include optional `mission_id` field for routing
- New API endpoints:
  - `GET /api/control/running` - List running missions
  - `GET /api/control/parallel/config` - Get parallel config
  - `POST /api/control/missions/:id/parallel` - Start in parallel
  - `POST /api/control/missions/:id/cancel` - Cancel specific mission

**Pending:**
- Dashboard UI for parallel mission management
- Full slot-based execution (currently uses simplified model)
- Queue reordering and priority

---

## 1. Progress Checkpoints / Milestones System

### Problem
Currently, parent agents (RootAgent, NodeAgent) have no way to:
- Monitor child progress in real-time
- Restart a task with a different prompt or model if unsatisfied
- Set intermediate milestones/checkpoints
- Implement "give up and try differently" logic

### Proposed Design

#### 1.1 Checkpoint Definition
```rust
/// A checkpoint that can be verified during task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// Verification criteria
    pub criteria: CheckpointCriteria,
    /// Deadline (iterations or time)
    pub deadline: Option<CheckpointDeadline>,
    /// What to do if checkpoint fails
    pub on_failure: CheckpointFailureAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointCriteria {
    /// File must exist at path
    FileExists(String),
    /// File must contain text
    FileContains { path: String, pattern: String },
    /// Tool call count threshold
    MinToolCalls(u32),
    /// Specific tool must have been called
    ToolCalled(String),
    /// Custom LLM verification
    LlmVerify { prompt: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointDeadline {
    Iterations(u32),
    Minutes(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointFailureAction {
    /// Continue anyway
    Continue,
    /// Retry with same config
    Retry { max_attempts: u32 },
    /// Upgrade model and retry
    UpgradeModel,
    /// Change prompt and retry
    ChangePrompt { new_prompt: String },
    /// Abort task
    Abort,
}
```

#### 1.2 Parent Agent Integration
```rust
impl NodeAgent {
    async fn execute_with_checkpoints(
        &self,
        task: &mut Task,
        checkpoints: Vec<Checkpoint>,
        ctx: &AgentContext,
    ) -> AgentResult {
        for checkpoint in &checkpoints {
            // Execute until checkpoint deadline
            let result = self.execute_until_checkpoint(task, checkpoint, ctx).await;
            
            match self.verify_checkpoint(checkpoint, &result, ctx).await {
                CheckpointResult::Passed => continue,
                CheckpointResult::Failed => {
                    match &checkpoint.on_failure {
                        CheckpointFailureAction::UpgradeModel => {
                            task.analysis_mut().requested_model = Some(self.get_upgrade_model());
                            return self.execute_with_checkpoints(task, checkpoints, ctx).await;
                        }
                        CheckpointFailureAction::ChangePrompt { new_prompt } => {
                            // Restart with modified prompt
                            task.set_description(new_prompt);
                            return self.execute_with_checkpoints(task, checkpoints, ctx).await;
                        }
                        // ... handle other actions
                    }
                }
            }
        }
        // All checkpoints passed
        self.finalize(task, ctx).await
    }
}
```

#### 1.3 Task-Level Checkpoints (User-Defined)
Add to task submission API:
```json
POST /api/control/message
{
  "content": "Analyze Rabby Wallet for security issues",
  "model": "x-ai/grok-4.1-fast",
  "checkpoints": [
    {
      "description": "Extension downloaded",
      "criteria": { "type": "file_exists", "path": "/root/work/*/rabby*.crx" },
      "deadline": { "iterations": 5 },
      "on_failure": "retry"
    },
    {
      "description": "Code extracted and indexed",
      "criteria": { "type": "tool_called", "name": "index_files" },
      "deadline": { "iterations": 15 },
      "on_failure": "upgrade_model"
    },
    {
      "description": "Audit report generated",
      "criteria": { "type": "file_contains", "path": "*/AUDIT_REPORT.md", "pattern": "## Findings" },
      "deadline": { "iterations": 40 },
      "on_failure": "abort"
    }
  ]
}
```

---

## 2. Parallel Missions with Queue Management

### Problem
- Missions run sequentially, blocking each other
- No way to manage the queue (reorder, delete, pause)
- No way to run missions in parallel
- No way to stop a running mission without cancelling everything

### Proposed API Design

#### 2.1 New Control API Endpoints

```
# Queue Management
GET    /api/control/queue                    # List queued messages
DELETE /api/control/queue/:id                # Remove from queue
PATCH  /api/control/queue/:id/position       # Reorder (move up/down)
POST   /api/control/queue/:id/priority       # Set priority (high/normal/low)

# Parallel Execution
POST   /api/control/message
{
  "content": "...",
  "model": "...",
  "parallel": true,          # Run immediately in parallel, don't queue
  "isolated": true           # Use separate context (no shared history)
}

# Mission Management
POST   /api/control/missions/:id/pause       # Pause a running mission
POST   /api/control/missions/:id/resume      # Resume a paused mission
POST   /api/control/missions/:id/cancel      # Cancel (stop execution)
DELETE /api/control/missions/:id             # Delete mission entirely

# Parallel Mission Slots
GET    /api/control/slots                    # List parallel execution slots
POST   /api/control/slots                    # Create new parallel slot
DELETE /api/control/slots/:id                # Remove slot
```

#### 2.2 Backend Changes

```rust
/// Execution slot for parallel missions
pub struct ExecutionSlot {
    pub id: Uuid,
    pub name: String,
    pub current_mission: Option<Uuid>,
    pub status: SlotStatus,
    pub cancel_token: CancellationToken,
}

pub enum SlotStatus {
    Idle,
    Running { mission_id: Uuid, started_at: DateTime },
    Paused { mission_id: Uuid },
}

/// Queue item with priority and metadata
pub struct QueueItem {
    pub id: Uuid,
    pub content: String,
    pub model: Option<String>,
    pub priority: Priority,
    pub created_at: DateTime,
    pub parallel: bool,
}

pub enum Priority {
    High,
    Normal,
    Low,
}
```

#### 2.3 UI/UX Design

```
┌─────────────────────────────────────────────────────────────┐
│ Mission Control                                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─ Active Slots ──────────────────────────────────────┐   │
│  │                                                      │   │
│  │  Slot 1: [■ Running] Security Audit - Grok         │   │
│  │  ├── Progress: 12/25 subtasks                       │   │
│  │  └── [Pause] [Cancel]                               │   │
│  │                                                      │   │
│  │  Slot 2: [■ Running] Code Analysis - Gemini        │   │
│  │  ├── Progress: 3/8 subtasks                         │   │
│  │  └── [Pause] [Cancel]                               │   │
│  │                                                      │   │
│  │  [+ Add Parallel Slot]                              │   │
│  │                                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ Queue (3 pending) ─────────────────────────────────┐   │
│  │                                                      │   │
│  │  1. 🔴 [HIGH] Fix auth bug - Claude                 │   │
│  │     └── [▲] [▼] [Run Now] [Delete]                  │   │
│  │                                                      │   │
│  │  2. ⚪ [NORMAL] Write tests - Qwen                  │   │
│  │     └── [▲] [▼] [Run Parallel] [Delete]             │   │
│  │                                                      │   │
│  │  3. ⚪ [NORMAL] Update docs                         │   │
│  │     └── [▲] [▼] [Run Parallel] [Delete]             │   │
│  │                                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.4 Keyboard Shortcuts
- `Ctrl+P` - Toggle parallel mode for new message
- `Ctrl+K` - Open queue management modal
- `Ctrl+Shift+C` - Cancel current slot
- `1-9` - Switch between slots

---

## 3. Optimized Security Audit Prompt

### Current Issues
1. Agent doesn't follow folder requirements
2. Scope creep to unrelated files (Vulcan instead of Rabby)
3. No final consolidated report
4. Too open-ended, explores indefinitely

### Optimized Prompt

```markdown
# Security Audit Task

## YOUR WORKING FOLDER (MANDATORY)
**ALL files you create MUST go in: `/root/work/security-audit-{your-model-name}/`**

Create this structure immediately:
```
/root/work/security-audit-{model}/
├── output/
│   └── AUDIT_REPORT.md    # Your final deliverable (REQUIRED)
├── temp/                   # Working files, downloads, extractions
└── notes.md               # Your analysis notes and findings
```

## TARGET
**Rabby Wallet Chrome Extension** - A cryptocurrency wallet with transaction simulation.

Source options:
1. Chrome Web Store: Extension ID `acmacodkjbdgmoleebolmdjonilkdbch`
2. GitHub: https://github.com/RabbyHub/Rabby
3. Pre-downloaded in `/root/context/` (check first)

## SCOPE - FOCUS ONLY ON THESE
1. **Transaction Simulation Bypass** - Can attackers make harmful transactions appear safe?
2. **Approval Amount Manipulation** - Can displayed approval amounts differ from actual?
3. **Spender Address Spoofing** - Can fake addresses be shown as trusted protocols?
4. **Permit2 Integration** - Validation of spender field against known reactors

## REFERENCE VULNERABILITY (Example of what to find)
A previous bug was found where Permit2 transactions could bypass simulation:
- Simulation showed "Spend 1 USDC to receive X"
- Actual transaction approved unlimited tokens to attacker
- Root cause: Spender field not validated against trusted addresses
- The witness data was trusted without verifying the spender

## METHODOLOGY
1. **FIRST**: Check `/root/context/` for existing files
2. Download/extract the extension if not present
3. Focus on: `background.js`, transaction simulation, Permit2 handling
4. Look for: Input validation gaps, trust assumptions, display vs. actual discrepancies
5. Document each finding with: Location, Description, Impact, PoC idea

## DELIVERABLE (REQUIRED)
Your FINAL message must contain the complete `AUDIT_REPORT.md` content:

```markdown
# Rabby Wallet Security Audit Report

**Auditor**: {model-name}
**Date**: {date}
**Scope**: Transaction simulation, Permit2, Approval handling

## Executive Summary
[2-3 sentences on overall security posture]

## Findings

### [CRITICAL/HIGH/MEDIUM/LOW] Finding 1: Title
- **Location**: `path/to/file.js:line`
- **Description**: What the issue is
- **Impact**: What an attacker could do
- **PoC Concept**: How to exploit
- **Recommendation**: How to fix

### Finding 2: ...

## Files Analyzed
[List of key files reviewed with notes]

## Methodology
[Tools and approach used]

## Conclusion
[Summary and recommendations]
```

## RULES
1. **Stay in your folder** - Don't create files elsewhere
2. **Ignore other files** - If you see Vulcan.jar or other unrelated files, IGNORE them
3. **Time-box exploration** - Spend max 10 tool calls on setup, then analyze
4. **Report must be in final message** - Not just a file path, the actual content
5. **Call complete_mission** when done - With status and summary
```

---

---

## 4. New Backlog Issues & Proposed Fixes

### 4.1 Model Override Not Persisted to Database

#### Problem
When a user submits a message with a model override, it's used during execution but NOT saved to the `missions` table. This means:
- Dashboard can't show which model was requested
- Mission history loses context
- Can't filter/search missions by model used

#### Current Flow
```
POST /api/control/message { content: "...", model: "grok-4.1-fast" }
    ↓
ControlCommand::UserMessage { id, content, model: Some("grok-4.1-fast") }
    ↓
run_single_control_turn(..., model_override: Some("grok-4.1-fast"), ...)
    ↓
task.analysis_mut().requested_model = Some(model)  ← Only in-memory!
    ↓
Mission saved to DB with model_override = NULL  ← BUG
```

#### Proposed Fix

**Option A: Add column to missions table (Recommended)**

```sql
-- Migration
ALTER TABLE missions ADD COLUMN model_override TEXT;
```

```rust
// src/memory/supabase.rs - Update create_mission
pub async fn create_mission(&self, title: Option<&str>, model_override: Option<&str>) -> Result<DbMission> {
    let body = json!({
        "title": title,
        "status": "active",
        "history": [],
        "model_override": model_override,  // NEW
    });
    // ...
}

// src/api/control.rs - Pass model to mission creation
async fn create_new_mission(memory: &Option<MemorySystem>, model: Option<&str>) -> Result<Mission, String> {
    let mem = memory.as_ref().ok_or("Memory not configured")?;
    let db_mission = mem.supabase.create_mission(None, model).await?;
    // ...
}
```

**Option B: Store in mission history metadata**
```rust
// Store model as first history entry metadata
let first_entry = MissionHistoryEntry {
    role: "system".to_string(),
    content: format!("Model override: {}", model),
};
```

#### API Changes
```rust
// Mission struct gets new field
#[derive(Serialize, Deserialize)]
pub struct Mission {
    pub id: Uuid,
    pub status: MissionStatus,
    pub title: Option<String>,
    pub model_override: Option<String>,  // NEW
    pub history: Vec<MissionHistoryEntry>,
    // ...
}
```

---

### 4.2 No Timeout for Dangerous Commands

#### Problem
Commands like `find /` can run indefinitely, blocking the agent loop:
```
find / -type f -name 'key.pem' 2>/dev/null  # Takes 4+ seconds, repeated 50+ times
```

The current timeout (60s) is per-command, but:
- Multiple slow commands accumulate
- No blacklist for known-dangerous patterns
- No smart interruption

#### Proposed Fix

**4.2.1 Command Pattern Blacklist**

```rust
// src/tools/terminal.rs

/// Commands that should be blocked or warned about
const DANGEROUS_PATTERNS: &[&str] = &[
    "find /",           // Full filesystem search
    "find / ",
    "grep -r /",        // Recursive grep from root
    "du -sh /",         // Disk usage from root
    "ls -laR /",        // Recursive listing from root
    "cat /dev/",        // Reading device files
    "rm -rf /",         // Obviously dangerous
    "dd if=/dev/",      // Disk operations
];

/// Commands that should use shorter timeouts
const SLOW_PATTERNS: &[(&str, u64)] = &[
    ("find ", 10_000),          // 10s max for find
    ("grep -r", 15_000),        // 15s max for recursive grep
    ("apt ", 120_000),          // 2min for apt
    ("cargo build", 300_000),   // 5min for cargo
];

pub fn validate_command(cmd: &str) -> Result<(), String> {
    for pattern in DANGEROUS_PATTERNS {
        if cmd.contains(pattern) {
            return Err(format!(
                "Blocked dangerous command pattern: '{}'. Use a more specific path.",
                pattern
            ));
        }
    }
    Ok(())
}

pub fn get_timeout_for_command(cmd: &str, default: u64) -> u64 {
    for (pattern, timeout) in SLOW_PATTERNS {
        if cmd.contains(pattern) {
            return *timeout;
        }
    }
    default
}
```

**4.2.2 Smart Alternatives Suggestion**

When blocking a command, suggest alternatives:
```rust
fn suggest_alternative(cmd: &str) -> Option<String> {
    if cmd.starts_with("find /") {
        Some("Use 'find /root/work/ ...' or 'find /specific/path ...' instead".to_string())
    } else if cmd.starts_with("grep -r /") {
        Some("Use 'grep -r /root/ ...' or specify a directory".to_string())
    } else {
        None
    }
}
```

**4.2.3 Cumulative Time Budget**

```rust
// Track total command time per task
pub struct CommandTimeBudget {
    total_allowed_ms: u64,
    spent_ms: u64,
}

impl CommandTimeBudget {
    pub fn can_run(&self, estimated_ms: u64) -> bool {
        self.spent_ms + estimated_ms <= self.total_allowed_ms
    }
    
    pub fn record(&mut self, elapsed_ms: u64) {
        self.spent_ms += elapsed_ms;
    }
}
```

---

### 4.3 No Working Folder Enforcement

#### Problem
The agent creates files everywhere:
```
/root/work/rabby-analysis/
/root/work/rabby-wallet-download/
/root/work/security-audit-grok-4/
/root/rabby_temp/                    # Wrong location!
/tmp/rabby_extract/                  # Also wrong!
```

#### Proposed Fix

**4.3.1 Task-Scoped Working Directory**

```rust
// src/task/mod.rs
pub struct Task {
    // ...
    /// Designated working folder for this task (enforced)
    pub working_folder: Option<PathBuf>,
}

impl Task {
    pub fn with_working_folder(mut self, folder: &str) -> Self {
        self.working_folder = Some(PathBuf::from(folder));
        self
    }
    
    /// Check if a path is within the allowed working folder
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        match &self.working_folder {
            Some(folder) => path.starts_with(folder) || path.starts_with("/root/context/"),
            None => true,  // No restriction if not set
        }
    }
}
```

**4.3.2 File Operation Validation**

```rust
// src/tools/files.rs

pub async fn write_file(
    path: &str,
    content: &str,
    task: Option<&Task>,
) -> Result<String, String> {
    let path = Path::new(path);
    
    // Validate path if task has working folder
    if let Some(task) = task {
        if !task.is_path_allowed(path) {
            return Err(format!(
                "File operation blocked: {} is outside the designated working folder {:?}. \
                 Create files in your task folder instead.",
                path.display(),
                task.working_folder
            ));
        }
    }
    
    // Proceed with write...
}
```

**4.3.3 Auto-Create Task Folder**

```rust
// In TaskExecutor, before starting execution
async fn setup_working_folder(&self, task: &mut Task, model_name: &str) {
    let folder_name = format!("security-audit-{}", 
        model_name.split('/').last().unwrap_or("unknown"));
    let folder_path = format!("/root/work/{}", folder_name);
    
    // Create folder structure
    std::fs::create_dir_all(format!("{}/output", folder_path)).ok();
    std::fs::create_dir_all(format!("{}/temp", folder_path)).ok();
    
    task.working_folder = Some(PathBuf::from(&folder_path));
    
    // Inject into system prompt
    tracing::info!("Task working folder set to: {}", folder_path);
}
```

---

### 4.4 Missing Parallel Execution

#### Problem
All missions run sequentially. One stuck mission blocks everything.

#### Proposed Fix (Expanded from Section 2)

**4.4.1 Execution Slots Architecture**

```rust
// src/api/control.rs

/// Multiple parallel execution slots
pub struct ParallelExecutor {
    slots: Vec<ExecutionSlot>,
    max_slots: usize,
    default_slot: usize,
}

pub struct ExecutionSlot {
    pub id: usize,
    pub name: String,
    pub queue: VecDeque<QueuedMessage>,
    pub current: Option<RunningTask>,
    pub cancel_token: Option<CancellationToken>,
}

impl ParallelExecutor {
    pub fn new(max_slots: usize) -> Self {
        Self {
            slots: vec![ExecutionSlot::new(0, "Main")],
            max_slots,
            default_slot: 0,
        }
    }
    
    /// Add a message to specific slot or create new parallel slot
    pub fn enqueue(&mut self, msg: QueuedMessage, parallel: bool) -> Result<usize, String> {
        if parallel {
            // Find idle slot or create new one
            if let Some(slot) = self.slots.iter_mut().find(|s| s.is_idle()) {
                slot.queue.push_back(msg);
                return Ok(slot.id);
            }
            
            if self.slots.len() < self.max_slots {
                let new_id = self.slots.len();
                let mut slot = ExecutionSlot::new(new_id, &format!("Slot {}", new_id));
                slot.queue.push_back(msg);
                self.slots.push(slot);
                return Ok(new_id);
            }
            
            Err("Max parallel slots reached".to_string())
        } else {
            // Queue to default slot
            self.slots[self.default_slot].queue.push_back(msg);
            Ok(self.default_slot)
        }
    }
}
```

**4.4.2 API Endpoints**

```rust
// New routes
router
    .route("/api/control/slots", get(list_slots).post(create_slot))
    .route("/api/control/slots/:id", delete(remove_slot))
    .route("/api/control/slots/:id/pause", post(pause_slot))
    .route("/api/control/slots/:id/resume", post(resume_slot))
    .route("/api/control/queue", get(list_queue))
    .route("/api/control/queue/:id", delete(remove_from_queue))
    .route("/api/control/queue/:id/position", patch(reorder_queue))
```

**4.4.3 Message Submission with Parallel Flag**

```rust
#[derive(Deserialize)]
pub struct ControlMessageRequest {
    pub content: String,
    pub model: Option<String>,
    #[serde(default)]
    pub parallel: bool,      // Run in parallel slot
    #[serde(default)]
    pub slot_id: Option<usize>,  // Specific slot to use
    #[serde(default)]
    pub priority: Priority,  // high/normal/low
}
```

---

### 4.5 No Checkpoint/Progress Gates

#### Problem
No way to:
- Monitor progress within a task
- Abort early if task is going off-track
- Set intermediate verification points

#### Proposed Fix (Expanded from Section 1)

**4.5.1 Lightweight Progress Reporting**

```rust
// Add to ExecutionSignals
pub struct ExecutionSignals {
    // ... existing fields ...
    
    /// Key milestones achieved
    pub milestones: Vec<Milestone>,
    
    /// Current phase description
    pub current_phase: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Milestone {
    pub name: String,
    pub achieved_at: DateTime<Utc>,
    pub iteration: u32,
}
```

**4.5.2 Tool-Based Milestone Reporting**

```rust
// New tool: report_progress
pub struct ReportProgressTool;

impl Tool for ReportProgressTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "report_progress".to_string(),
            description: "Report progress on the current task. Use this to signal milestones.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "milestone": {
                        "type": "string",
                        "description": "Name of the milestone achieved (e.g., 'downloaded_extension', 'extracted_code', 'found_vulnerability')"
                    },
                    "phase": {
                        "type": "string",
                        "description": "Current phase of work (e.g., 'setup', 'analysis', 'reporting')"
                    },
                    "progress_percent": {
                        "type": "integer",
                        "description": "Estimated progress 0-100"
                    }
                },
                "required": ["milestone"]
            }),
        }
    }
}
```

**4.5.3 Parent Agent Progress Monitoring**

```rust
impl NodeAgent {
    async fn execute_with_monitoring(
        &self,
        task: &mut Task,
        ctx: &AgentContext,
    ) -> AgentResult {
        let start = Instant::now();
        let max_duration = Duration::from_secs(300); // 5 min per subtask
        
        loop {
            // Check time budget
            if start.elapsed() > max_duration {
                tracing::warn!("Subtask exceeded time budget, considering restart");
                return self.handle_timeout(task, ctx).await;
            }
            
            // Check progress
            let signals = self.get_current_signals(task);
            if signals.iterations > 20 && signals.milestones.is_empty() {
                tracing::warn!("No milestones after 20 iterations, considering model upgrade");
                return self.handle_no_progress(task, ctx).await;
            }
            
            // Continue execution...
        }
    }
    
    async fn handle_no_progress(&self, task: &mut Task, ctx: &AgentContext) -> AgentResult {
        // Options:
        // 1. Upgrade model
        // 2. Simplify prompt
        // 3. Abort and report failure
        
        let current_model = task.analysis().selected_model.as_deref();
        if let Some(upgrade) = self.get_model_upgrade(current_model) {
            tracing::info!("Upgrading model from {:?} to {}", current_model, upgrade);
            task.analysis_mut().requested_model = Some(upgrade);
            return self.execute(task, ctx).await;  // Retry with new model
        }
        
        AgentResult::failure("Task made no progress after 20 iterations", 0)
    }
}
```

---

## 5. Implementation Priority

### 🔴 Immediate (Deploy Now)
- [x] Fix model override bug (user-requested models bypass allowlist)
- [x] Improve system prompt for deliverables
- [ ] **Deploy pending fixes to production**

### 🟠 High Priority (This Week)
| Task | Effort | Impact | Files |
|------|--------|--------|-------|
| Persist model_override to DB | 2h | High | `supabase.rs`, `control.rs`, SQL migration |
| Add command pattern blacklist | 1h | High | `terminal.rs` |
| Add dynamic command timeout | 1h | Medium | `terminal.rs` |
| Clean workspace tool | 30m | Medium | New tool |

### 🟡 Medium Priority (Next Sprint)
| Task | Effort | Impact | Files |
|------|--------|--------|-------|
| Working folder enforcement | 3h | High | `task/mod.rs`, `files.rs`, `terminal.rs` |
| Queue management endpoints | 4h | Medium | `control.rs`, new routes |
| Parallel execution (2 slots) | 6h | High | `control.rs`, major refactor |
| Pause/resume missions | 2h | Medium | `control.rs` |
| Progress reporting tool | 2h | Medium | New tool |

### 🟢 Low Priority (Future)
| Task | Effort | Impact | Files |
|------|--------|--------|-------|
| Full checkpoint system | 2d | High | New module |
| Parent agent retry logic | 1d | Medium | `node.rs`, `root.rs` |
| Parallel slots UI | 1d | Medium | Dashboard |
| Mission templates | 4h | Low | Config files |

### 🔵 Research/Long-term
- Automatic checkpoint inference from task description
- Learning-based retry strategy selection
- Cross-mission context sharing for related tasks
- Agent self-reflection and strategy adjustment

---

## 6. Quick Wins (Can Do Now)

These fixes can be implemented immediately with minimal risk:

### 6.1 Command Blacklist (5 min)

Add to `src/tools/terminal.rs`:

```rust
fn validate_command(cmd: &str) -> Result<(), String> {
    let dangerous = [
        ("find /", "Use 'find /root/work/' instead"),
        ("find / ", "Use 'find /root/work/' instead"),
        ("grep -r /", "Use 'grep -r /root/' instead"),
        ("rm -rf /", "This would destroy the system"),
    ];
    
    for (pattern, suggestion) in dangerous {
        if cmd.trim().starts_with(pattern) || cmd.contains(&format!(" {}", pattern)) {
            return Err(format!("Blocked: '{}'. {}", pattern, suggestion));
        }
    }
    Ok(())
}
```

### 6.2 Model Override Persistence (15 min)

SQL migration:
```sql
ALTER TABLE missions ADD COLUMN model_override TEXT;
```

Update `create_mission` to accept and store model.

### 6.3 Workspace Cleanup Tool (10 min)

Add new tool `clean_workspace`:
```rust
pub struct CleanWorkspaceTool;

impl Tool for CleanWorkspaceTool {
    fn execute(&self, args: Value) -> Result<String> {
        let older_than_days: u64 = args["older_than_days"].as_u64().unwrap_or(7);
        let dry_run = args["dry_run"].as_bool().unwrap_or(true);
        
        // Find folders in /root/work/ older than N days
        // Delete or report what would be deleted
    }
}
```

---

## 7. Context Isolation & Workspace Management

### Problem
The current architecture allows context pollution across missions:
1. **Shared `/root/context/`** - All missions read/write to the same folder
2. **No cleanup** - Previous mission files remain and confuse new missions
3. **No source tracking** - Agent forgets where it downloaded sources
4. **Work folder not enforced** - Agent can analyze files outside its work folder

### Real-World Failure (Dec 19, 2025)
A Rabby Wallet security audit mission:
- Found Vulcan anti-cheat files in `/root/context/` from a previous mission
- Rabby CRX extraction failed silently (`rabby_wallet_extracted/` was empty)
- Agent pivoted to analyzing Vulcan instead of Rabby
- Produced "Vulcan Anti-Cheat Security Audit Report" instead of Rabby report

### Proposed Solutions

#### 7.1 Mission-Specific Context Subfolders

```rust
// Add to Mission struct
pub struct Mission {
    pub id: Uuid,
    pub title: Option<String>,
    pub status: String,
    pub context_subfolder: Option<String>, // NEW: e.g., "rabby-audit-20251219"
    // ...
}
```

When creating a mission, generate a unique context path:
```rust
let context_subfolder = format!("{}-{}", sanitize(title), mission_id.to_string()[..8]);
// Results in: /root/context/security-audit-rabby-3f6979b4/
```

Inject into system prompt:
```
Your mission context files are in: /root/context/security-audit-rabby-3f6979b4/
Your work folder is: /root/work/security-audit-grok/

Only analyze files within these two folders.
Do NOT access /root/context/ directly.
```

#### 7.2 Mandatory Source Setup Subtask

Modify RootAgent task splitting to ALWAYS start with a setup phase:

```rust
// In root.rs, when splitting tasks
fn create_mandatory_setup_subtask(&self, task: &Task) -> Subtask {
    Subtask {
        id: "setup".to_string(),
        description: format!(
            "Setup Phase: Create working directory at {}/. \
             Acquire all source files needed for analysis INTO this folder. \
             Do NOT use /root/context/. \
             Create notes/sources.md documenting what you downloaded and from where.",
            self.get_work_folder()
        ),
        dependencies: vec![],
        priority: 0, // Always first
        is_mandatory: true,
    }
}
```

#### 7.3 Work Folder Enforcement in Tools

Add validation to terminal/file tools:

```rust
// In tools/terminal.rs
fn validate_path(&self, path: &str) -> Result<(), ToolError> {
    let work_folder = self.mission_context.work_folder();
    let context_folder = self.mission_context.context_folder();
    
    // Allow reads from work folder and mission-specific context
    if path.starts_with(work_folder) || path.starts_with(context_folder) {
        return Ok(());
    }
    
    // Allow reads from common system paths
    if path.starts_with("/usr/") || path.starts_with("/bin/") {
        return Ok(());
    }
    
    // Block access to other mission contexts
    if path.starts_with("/root/context/") {
        return Err(ToolError::new(format!(
            "Access denied: {} is outside your mission context. Use {} instead.",
            path, context_folder
        )));
    }
    
    Ok(()) // Allow other paths (might need to clone repos, etc.)
}
```

#### 7.4 Context Cleanup on Mission Complete

```rust
// In control.rs, when mission completes
async fn complete_mission(&mut self, mission_id: Uuid, status: &str) {
    // ... existing completion logic ...
    
    // Optional: Archive and clean context folder
    if self.config.auto_clean_context {
        let context_path = format!("/root/context/{}", mission.context_subfolder);
        let archive_path = format!("/root/archive/{}", mission.context_subfolder);
        
        // Move to archive instead of delete
        tokio::fs::rename(&context_path, &archive_path).await.ok();
    }
}
```

#### 7.5 Source Manifest Requirement

The setup subtask should produce a manifest:

```markdown
# Source Manifest (/root/work/security-audit-grok/notes/sources.md)

## Acquired Sources
| Source | Location | Method | Verified |
|--------|----------|--------|----------|
| Rabby Wallet | ./source/ | git clone github.com/RabbyHub/Rabby | ✅ Yes (package.json exists) |

## Key Directories
- `./source/src/background/` - Extension background logic
- `./source/_raw/` - Built extension assets

## Files Indexed
- Total: 1,234 files
- JavaScript: 456
- TypeScript: 234
- JSON: 89

## Analysis Scope
Only files within `/root/work/security-audit-grok/` will be analyzed.
```

### Improved Prompt Template

See `scripts/prompts/security_audit_v2.md` which implements:
1. Mandatory workspace setup BEFORE any analysis
2. Clone sources directly INTO the work folder (not /root/context/)
3. Explicit FORBIDDEN section blocking /root/context/ access
4. Source manifest requirement
5. Verification step before proceeding

### Migration Path

1. **Immediate (prompt-level fix)**: Use v2 prompt that clones to work folder
2. **Short-term**: Add `context_subfolder` to Mission
3. **Medium-term**: Add path validation to tools
4. **Long-term**: Full context isolation with cleanup

---

## 8. Testing Checklist

Before rerunning the security audit experiment:

### Pre-Deployment
- [ ] Deploy Gemini thought_signature fix
- [ ] Deploy model override fix
- [ ] Deploy system prompt improvements  
- [ ] Deploy command blacklist

### Context Cleanup
- [ ] Clean work folder: `ssh root@95.216.112.253 'rm -rf /root/work/*'`
- [ ] Clean context folder: `ssh root@95.216.112.253 'rm -rf /root/context/*'`
- [ ] Or archive: `ssh root@95.216.112.253 'mv /root/context /root/archive/context-$(date +%Y%m%d) && mkdir /root/context'`

### Prompt Selection
- [ ] Use v2 prompt: `scripts/prompts/security_audit_v2.md`
- [ ] Verify prompt instructs agent to clone INTO work folder
- [ ] Verify prompt FORBIDS reading /root/context/

### Execution
- [ ] Start with 1-2 models first (recommend: grok, qwen)
- [ ] Wait for first mission to complete setup phase
- [ ] Verify sources are in `/root/work/security-audit-{model}/source/`
- [ ] Monitor for 10 minutes before leaving unattended

### Validation
- [ ] Check that AUDIT_REPORT.md mentions "Rabby" not "Vulcan"
- [ ] Check sources.md manifest was created
- [ ] Verify no analysis of `/root/context/` files
