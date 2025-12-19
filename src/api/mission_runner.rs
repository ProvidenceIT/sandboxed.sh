//! Mission Runner - Isolated execution context for a single mission.
//!
//! This module provides a clean abstraction for running missions in parallel.
//! Each MissionRunner manages its own:
//! - Conversation history
//! - Message queue  
//! - Execution state
//! - Cancellation token

use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agents::{AgentContext, AgentRef, AgentResult};
use crate::budget::{Budget, ModelPricing, SharedBenchmarkRegistry, SharedModelResolver};
use crate::config::Config;
use crate::llm::OpenRouterClient;
use crate::memory::{ContextBuilder, MemorySystem};
use crate::task::VerificationCriteria;
use crate::tools::ToolRegistry;

use super::control::{
    AgentEvent, AgentTreeNode, ControlStatus, ExecutionProgress, FrontendToolHub,
};

/// State of a running mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRunState {
    /// Waiting in queue
    Queued,
    /// Currently executing
    Running,
    /// Waiting for frontend tool input
    WaitingForTool,
    /// Finished (check result)
    Finished,
}

/// A message queued for this mission.
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub id: Uuid,
    pub content: String,
    pub model_override: Option<String>,
}

/// Isolated runner for a single mission.
pub struct MissionRunner {
    /// Mission ID
    pub mission_id: Uuid,
    
    /// Model override for this mission (if any)
    pub model_override: Option<String>,
    
    /// Current state
    pub state: MissionRunState,
    
    /// Message queue for this mission
    pub queue: VecDeque<QueuedMessage>,
    
    /// Conversation history: (role, content)
    pub history: Vec<(String, String)>,
    
    /// Cancellation token for the current execution
    pub cancel_token: Option<CancellationToken>,
    
    /// Running task handle
    running_handle: Option<tokio::task::JoinHandle<(Uuid, String, AgentResult)>>,
    
    /// Tree snapshot for this mission
    pub tree_snapshot: Arc<RwLock<Option<AgentTreeNode>>>,
    
    /// Progress snapshot for this mission
    pub progress_snapshot: Arc<RwLock<ExecutionProgress>>,
}

impl MissionRunner {
    /// Create a new mission runner.
    pub fn new(mission_id: Uuid, model_override: Option<String>) -> Self {
        Self {
            mission_id,
            model_override,
            state: MissionRunState::Queued,
            queue: VecDeque::new(),
            history: Vec::new(),
            cancel_token: None,
            running_handle: None,
            tree_snapshot: Arc::new(RwLock::new(None)),
            progress_snapshot: Arc::new(RwLock::new(ExecutionProgress::default())),
        }
    }
    
    /// Check if this runner is currently executing.
    pub fn is_running(&self) -> bool {
        matches!(self.state, MissionRunState::Running | MissionRunState::WaitingForTool)
    }
    
    /// Check if this runner has finished.
    pub fn is_finished(&self) -> bool {
        matches!(self.state, MissionRunState::Finished)
    }
    
    /// Queue a message for this mission.
    pub fn queue_message(&mut self, id: Uuid, content: String, model_override: Option<String>) {
        self.queue.push_back(QueuedMessage {
            id,
            content,
            model_override: model_override.or_else(|| self.model_override.clone()),
        });
    }
    
    /// Cancel the current execution.
    pub fn cancel(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
    }
    
    /// Start executing the next queued message (if any and not already running).
    /// Returns true if execution was started.
    pub fn start_next(
        &mut self,
        config: Config,
        root_agent: AgentRef,
        memory: Option<MemorySystem>,
        benchmarks: SharedBenchmarkRegistry,
        resolver: SharedModelResolver,
        pricing: Arc<ModelPricing>,
        events_tx: broadcast::Sender<AgentEvent>,
        tool_hub: Arc<FrontendToolHub>,
        status: Arc<RwLock<ControlStatus>>,
        mission_cmd_tx: mpsc::Sender<crate::tools::mission::MissionControlCommand>,
        current_mission: Arc<RwLock<Option<Uuid>>>,
    ) -> bool {
        // Don't start if already running
        if self.is_running() {
            return false;
        }
        
        // Get next message from queue
        let msg = match self.queue.pop_front() {
            Some(m) => m,
            None => return false,
        };
        
        self.state = MissionRunState::Running;
        
        let cancel = CancellationToken::new();
        self.cancel_token = Some(cancel.clone());
        
        let hist_snapshot = self.history.clone();
        let tree_ref = Arc::clone(&self.tree_snapshot);
        let progress_ref = Arc::clone(&self.progress_snapshot);
        let mission_id = self.mission_id;
        let model_override = msg.model_override;
        let user_message = msg.content.clone();
        let msg_id = msg.id;
        
        // Create mission control for complete_mission tool
        let mission_ctrl = crate::tools::mission::MissionControl {
            current_mission_id: current_mission,
            cmd_tx: mission_cmd_tx,
        };
        
        // Emit user message event with mission context
        let _ = events_tx.send(AgentEvent::UserMessage {
            id: msg_id,
            content: user_message.clone(),
            mission_id: Some(mission_id),
        });
        
        let handle = tokio::spawn(async move {
            let result = run_mission_turn(
                config,
                root_agent,
                memory,
                benchmarks,
                resolver,
                pricing,
                events_tx,
                tool_hub,
                status,
                cancel,
                hist_snapshot,
                user_message.clone(),
                model_override,
                Some(mission_ctrl),
                tree_ref,
                progress_ref,
                mission_id,
            )
            .await;
            (msg_id, user_message, result)
        });
        
        self.running_handle = Some(handle);
        true
    }
    
    /// Poll for completion. Returns Some(result) if finished.
    pub async fn poll_completion(&mut self) -> Option<(Uuid, String, AgentResult)> {
        let handle = self.running_handle.take()?;
        
        // Check if handle is finished
        if handle.is_finished() {
            match handle.await {
                Ok(result) => {
                    self.state = MissionRunState::Queued; // Ready for next message
                    // Add to history
                    self.history.push(("user".to_string(), result.1.clone()));
                    self.history.push(("assistant".to_string(), result.2.output.clone()));
                    Some(result)
                }
                Err(e) => {
                    tracing::error!("Mission runner task failed: {}", e);
                    self.state = MissionRunState::Finished;
                    None
                }
            }
        } else {
            // Not finished, put handle back
            self.running_handle = Some(handle);
            None
        }
    }
    
    /// Check if the running task is finished (non-blocking).
    pub fn check_finished(&self) -> bool {
        self.running_handle
            .as_ref()
            .map(|h| h.is_finished())
            .unwrap_or(true)
    }
}

/// Execute a single turn for a mission.
async fn run_mission_turn(
    config: Config,
    root_agent: AgentRef,
    memory: Option<MemorySystem>,
    benchmarks: SharedBenchmarkRegistry,
    resolver: SharedModelResolver,
    pricing: Arc<ModelPricing>,
    events_tx: broadcast::Sender<AgentEvent>,
    tool_hub: Arc<FrontendToolHub>,
    status: Arc<RwLock<ControlStatus>>,
    cancel: CancellationToken,
    history: Vec<(String, String)>,
    user_message: String,
    model_override: Option<String>,
    mission_control: Option<crate::tools::mission::MissionControl>,
    tree_snapshot: Arc<RwLock<Option<AgentTreeNode>>>,
    progress_snapshot: Arc<RwLock<ExecutionProgress>>,
    _mission_id: Uuid,
) -> AgentResult {
    // Build context with history
    let working_dir = config.working_dir.to_string_lossy().to_string();
    let context_builder = ContextBuilder::new(&config.context, &working_dir);
    let history_context = context_builder.build_history_context(&history);

    let mut convo = String::new();
    convo.push_str(&history_context);
    convo.push_str("User:\n");
    convo.push_str(&user_message);
    convo.push_str("\n\nInstructions:\n- Continue the conversation helpfully.\n- You may use tools to gather information or make changes.\n- When appropriate, use Tool UI tools (ui_*) for structured output or to ask for user selections.\n- For large data processing tasks (>10KB), use run_command to execute Python scripts rather than processing inline.\n- When you have fully completed the user's goal or determined it cannot be completed, use the complete_mission tool to mark the mission status.\n");

    let budget = Budget::new(1000);
    let verification = VerificationCriteria::None;
    let mut task = match crate::task::Task::new(convo, verification, budget) {
        Ok(t) => t,
        Err(e) => {
            return AgentResult::failure(format!("Failed to create task: {}", e), 0);
        }
    };

    // Apply model override if specified
    if let Some(model) = model_override {
        tracing::info!("Mission using model override: {}", model);
        task.analysis_mut().requested_model = Some(model);
    }

    // Create LLM client
    let llm = Arc::new(OpenRouterClient::new(config.api_key.clone()));

    // Create shared memory reference for memory tools
    let shared_memory: Option<crate::tools::memory::SharedMemory> = memory
        .as_ref()
        .map(|m| Arc::new(tokio::sync::RwLock::new(Some(m.clone()))));

    let tools = ToolRegistry::with_options(mission_control.clone(), shared_memory);
    let mut ctx = AgentContext::with_memory(
        config.clone(),
        llm,
        tools,
        pricing,
        config.working_dir.clone(),
        memory,
    );
    ctx.mission_control = mission_control;
    ctx.control_events = Some(events_tx);
    ctx.frontend_tool_hub = Some(tool_hub);
    ctx.control_status = Some(status);
    ctx.cancel_token = Some(cancel);
    ctx.benchmarks = Some(benchmarks);
    ctx.resolver = Some(resolver);
    ctx.tree_snapshot = Some(tree_snapshot);
    ctx.progress_snapshot = Some(progress_snapshot);

    root_agent.execute(&mut task, &ctx).await
}

/// Compact info about a running mission (for API responses).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunningMissionInfo {
    pub mission_id: Uuid,
    pub model_override: Option<String>,
    pub state: String,
    pub queue_len: usize,
    pub history_len: usize,
}

impl From<&MissionRunner> for RunningMissionInfo {
    fn from(runner: &MissionRunner) -> Self {
        Self {
            mission_id: runner.mission_id,
            model_override: runner.model_override.clone(),
            state: match runner.state {
                MissionRunState::Queued => "queued".to_string(),
                MissionRunState::Running => "running".to_string(),
                MissionRunState::WaitingForTool => "waiting_for_tool".to_string(),
                MissionRunState::Finished => "finished".to_string(),
            },
            queue_len: runner.queue.len(),
            history_len: runner.history.len(),
        }
    }
}
