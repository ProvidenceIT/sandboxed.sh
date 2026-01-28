/**
 * API Module Index
 * 
 * Re-exports all API functions and types for backward compatibility.
 * New code should import from specific modules when possible:
 * 
 * @example
 * // Preferred: Import from specific module
 * import { listMissions } from '@/lib/api/missions';
 * 
 * // Still works: Import from index
 * import { listMissions } from '@/lib/api';
 */

// Core utilities
export {
  apiUrl,
  isNetworkError,
  LibraryUnavailableError,
  apiFetch,
  apiGet,
  apiPost,
  apiPut,
  apiPatch,
  apiDel,
  libGet,
  libPost,
  libPut,
  libDel,
  ensureLibraryResponse,
} from "./core";

// Missions
export {
  type MissionStatus,
  type MissionHistoryEntry,
  type DesktopSessionInfo,
  type Mission,
  type StoredEvent,
  type CreateMissionOptions,
  type RunningMissionInfo,
  listMissions,
  getMission,
  getMissionEvents,
  getCurrentMission,
  createMission,
  loadMission,
  getRunningMissions,
  startMissionParallel,
  cancelMission,
  setMissionStatus,
  deleteMission,
  cleanupEmptyMissions,
  resumeMission,
} from "./missions";

// Workspaces
export {
  type WorkspaceType,
  type WorkspaceStatus,
  type Workspace,
  type ContainerDistro,
  CONTAINER_DISTROS,
  type WorkspaceDebugInfo,
  type InitLogResponse,
  listWorkspaces,
  getWorkspace,
  createWorkspace,
  updateWorkspace,
  syncWorkspace,
  deleteWorkspace,
  buildWorkspace,
  getWorkspaceDebug,
  getWorkspaceInitLog,
} from "./workspaces";

// Providers
export {
  type AIProviderType,
  type AIProviderTypeInfo,
  type AIProviderStatus,
  type AIProviderAuthMethod,
  type AIProvider,
  type AIProviderAuthResponse,
  type OAuthAuthorizeResponse,
  type BackendProviderResponse,
  type ProviderModel,
  type Provider,
  type ProvidersResponse,
  listAIProviders,
  listAIProviderTypes,
  getAIProvider,
  createAIProvider,
  updateAIProvider,
  deleteAIProvider,
  getProviderForBackend,
  authenticateAIProvider,
  setDefaultAIProvider,
  getAuthMethods,
  oauthAuthorize,
  oauthCallback,
  listProviders,
} from "./providers";
