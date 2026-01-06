'use client';

import { useState, useEffect, useCallback } from 'react';
import { toast } from 'sonner';
import {
  getHealth,
  HealthResponse,
  listOpenCodeConnections,
  createOpenCodeConnection,
  updateOpenCodeConnection,
  deleteOpenCodeConnection,
  testOpenCodeConnection,
  setDefaultOpenCodeConnection,
  OpenCodeConnection,
} from '@/lib/api';
import {
  Server,
  Save,
  RefreshCw,
  AlertTriangle,
  GitBranch,
  Zap,
  Plus,
  Trash2,
  Check,
  X,
  Star,
  ExternalLink,
  Loader,
} from 'lucide-react';
import { readSavedSettings, writeSavedSettings } from '@/lib/settings';
import { cn } from '@/lib/utils';

export default function SettingsPage() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [healthLoading, setHealthLoading] = useState(true);
  const [testingConnection, setTestingConnection] = useState(false);

  // Form state
  const [apiUrl, setApiUrl] = useState(
    () => readSavedSettings().apiUrl ?? 'http://127.0.0.1:3000'
  );
  const [libraryRepo, setLibraryRepo] = useState(
    () => readSavedSettings().libraryRepo ?? ''
  );

  // Track original values for unsaved changes
  const [originalValues, setOriginalValues] = useState({
    apiUrl: readSavedSettings().apiUrl ?? 'http://127.0.0.1:3000',
    libraryRepo: readSavedSettings().libraryRepo ?? '',
  });

  // Validation state
  const [urlError, setUrlError] = useState<string | null>(null);
  const [repoError, setRepoError] = useState<string | null>(null);

  // OpenCode connections state
  const [connections, setConnections] = useState<OpenCodeConnection[]>([]);
  const [connectionsLoading, setConnectionsLoading] = useState(true);
  const [showNewConnection, setShowNewConnection] = useState(false);
  const [newConnection, setNewConnection] = useState({
    name: '',
    base_url: 'http://127.0.0.1:4096',
    agent: '',
    permissive: true,
  });
  const [savingConnection, setSavingConnection] = useState(false);
  const [testingConnectionId, setTestingConnectionId] = useState<string | null>(null);
  const [editingConnection, setEditingConnection] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<Partial<OpenCodeConnection>>({});

  // Check if there are unsaved changes
  const hasUnsavedChanges =
    apiUrl !== originalValues.apiUrl || libraryRepo !== originalValues.libraryRepo;

  // Validate URL
  const validateUrl = useCallback((url: string) => {
    if (!url.trim()) {
      setUrlError('API URL is required');
      return false;
    }
    try {
      new URL(url);
      setUrlError(null);
      return true;
    } catch {
      setUrlError('Invalid URL format');
      return false;
    }
  }, []);

  const validateRepo = useCallback((repo: string) => {
    const trimmed = repo.trim();
    if (!trimmed) {
      setRepoError(null);
      return true;
    }
    if (/\s/.test(trimmed)) {
      setRepoError('Repository URL cannot contain spaces');
      return false;
    }
    setRepoError(null);
    return true;
  }, []);

  // Load health and connections on mount
  useEffect(() => {
    const checkHealth = async () => {
      setHealthLoading(true);
      try {
        const data = await getHealth();
        setHealth(data);
      } catch {
        setHealth(null);
      } finally {
        setHealthLoading(false);
      }
    };
    checkHealth();
    loadConnections();
  }, []);

  const loadConnections = async () => {
    try {
      setConnectionsLoading(true);
      const data = await listOpenCodeConnections();
      setConnections(data);
    } catch {
      // Silent fail - connections might not be available yet
    } finally {
      setConnectionsLoading(false);
    }
  };

  // Unsaved changes warning
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (hasUnsavedChanges) {
        e.preventDefault();
        e.returnValue = '';
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [hasUnsavedChanges]);

  // Keyboard shortcut to save (Ctrl/Cmd + S)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [apiUrl, libraryRepo]);

  const handleSave = () => {
    const urlValid = validateUrl(apiUrl);
    const repoValid = validateRepo(libraryRepo);

    if (!urlValid || !repoValid) {
      toast.error('Please fix validation errors before saving');
      return;
    }

    writeSavedSettings({ apiUrl, libraryRepo });
    setOriginalValues({ apiUrl, libraryRepo });
    toast.success('Settings saved!');
  };

  const testApiConnection = async () => {
    if (!validateUrl(apiUrl)) {
      toast.error('Please enter a valid API URL');
      return;
    }

    setTestingConnection(true);
    try {
      const response = await fetch(`${apiUrl}/api/health`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      setHealth(data);
      toast.success(`Connected to OpenAgent v${data.version}`);
    } catch (err) {
      setHealth(null);
      toast.error(
        `Connection failed: ${err instanceof Error ? err.message : 'Unknown error'}`
      );
    } finally {
      setTestingConnection(false);
    }
  };

  const handleCreateConnection = async () => {
    if (!newConnection.name.trim()) {
      toast.error('Name is required');
      return;
    }
    if (!newConnection.base_url.trim()) {
      toast.error('Base URL is required');
      return;
    }

    try {
      new URL(newConnection.base_url);
    } catch {
      toast.error('Invalid URL format');
      return;
    }

    setSavingConnection(true);
    try {
      await createOpenCodeConnection({
        name: newConnection.name,
        base_url: newConnection.base_url,
        agent: newConnection.agent || null,
        permissive: newConnection.permissive,
      });
      toast.success('Connection created');
      setShowNewConnection(false);
      setNewConnection({
        name: '',
        base_url: 'http://127.0.0.1:4096',
        agent: '',
        permissive: true,
      });
      loadConnections();
    } catch (err) {
      toast.error(
        `Failed to create connection: ${err instanceof Error ? err.message : 'Unknown error'}`
      );
    } finally {
      setSavingConnection(false);
    }
  };

  const handleTestConnection = async (id: string) => {
    setTestingConnectionId(id);
    try {
      const result = await testOpenCodeConnection(id);
      if (result.success) {
        toast.success(result.message + (result.version ? ` (v${result.version})` : ''));
      } else {
        toast.error(result.message);
      }
    } catch (err) {
      toast.error(
        `Test failed: ${err instanceof Error ? err.message : 'Unknown error'}`
      );
    } finally {
      setTestingConnectionId(null);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await setDefaultOpenCodeConnection(id);
      toast.success('Default connection updated');
      loadConnections();
    } catch (err) {
      toast.error(
        `Failed to set default: ${err instanceof Error ? err.message : 'Unknown error'}`
      );
    }
  };

  const handleDeleteConnection = async (id: string) => {
    try {
      await deleteOpenCodeConnection(id);
      toast.success('Connection deleted');
      loadConnections();
    } catch (err) {
      toast.error(
        `Failed to delete: ${err instanceof Error ? err.message : 'Unknown error'}`
      );
    }
  };

  const handleStartEdit = (conn: OpenCodeConnection) => {
    setEditingConnection(conn.id);
    setEditForm({
      name: conn.name,
      base_url: conn.base_url,
      agent: conn.agent,
      permissive: conn.permissive,
      enabled: conn.enabled,
    });
  };

  const handleSaveEdit = async () => {
    if (!editingConnection) return;

    try {
      await updateOpenCodeConnection(editingConnection, editForm);
      toast.success('Connection updated');
      setEditingConnection(null);
      loadConnections();
    } catch (err) {
      toast.error(
        `Failed to update: ${err instanceof Error ? err.message : 'Unknown error'}`
      );
    }
  };

  const handleCancelEdit = () => {
    setEditingConnection(null);
    setEditForm({});
  };

  return (
    <div className="min-h-screen flex flex-col items-center p-6">
      {/* Centered content container */}
      <div className="w-full max-w-xl">
        {/* Header */}
        <div className="mb-8 flex items-start justify-between">
          <div>
            <h1 className="text-xl font-semibold text-white">Settings</h1>
            <p className="mt-1 text-sm text-white/50">
              Configure your server connection and preferences
            </p>
          </div>
          {hasUnsavedChanges && (
            <div className="flex items-center gap-2 text-amber-400 text-xs">
              <AlertTriangle className="h-3.5 w-3.5" />
              <span>Unsaved changes</span>
            </div>
          )}
        </div>

        <div className="space-y-5">
          {/* API Connection */}
          <div className="rounded-xl bg-white/[0.02] border border-white/[0.04] p-5">
            <div className="flex items-center gap-3 mb-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-500/10">
                <Server className="h-5 w-5 text-indigo-400" />
              </div>
              <div>
                <h2 className="text-sm font-medium text-white">API Connection</h2>
                <p className="text-xs text-white/40">Configure server endpoint</p>
              </div>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-white/60 mb-1.5">
                  API URL
                </label>
                <input
                  type="text"
                  value={apiUrl}
                  onChange={(e) => {
                    setApiUrl(e.target.value);
                    validateUrl(e.target.value);
                  }}
                  className={cn(
                    'w-full rounded-lg border bg-white/[0.02] px-3 py-2.5 text-sm text-white placeholder-white/30 focus:outline-none transition-colors',
                    urlError
                      ? 'border-red-500/50 focus:border-red-500/50'
                      : 'border-white/[0.06] focus:border-indigo-500/50'
                  )}
                />
                {urlError && <p className="mt-1.5 text-xs text-red-400">{urlError}</p>}
              </div>

              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-white/40">Status:</span>
                  {healthLoading ? (
                    <span className="flex items-center gap-1.5 text-xs text-white/40">
                      <RefreshCw className="h-3 w-3 animate-spin" />
                      Checking...
                    </span>
                  ) : health ? (
                    <span className="flex items-center gap-1.5 text-xs text-emerald-400">
                      <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                      Connected (v{health.version})
                    </span>
                  ) : (
                    <span className="flex items-center gap-1.5 text-xs text-red-400">
                      <span className="h-1.5 w-1.5 rounded-full bg-red-400" />
                      Disconnected
                    </span>
                  )}
                </div>

                <button
                  onClick={testApiConnection}
                  disabled={testingConnection}
                  className="flex items-center gap-2 rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-1.5 text-xs text-white/70 hover:bg-white/[0.04] transition-colors disabled:opacity-50"
                >
                  <RefreshCw
                    className={cn('h-3 w-3', testingConnection && 'animate-spin')}
                  />
                  Test Connection
                </button>
              </div>
            </div>
          </div>

          {/* OpenCode Connections */}
          <div className="rounded-xl bg-white/[0.02] border border-white/[0.04] p-5">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-violet-500/10">
                  <Zap className="h-5 w-5 text-violet-400" />
                </div>
                <div>
                  <h2 className="text-sm font-medium text-white">OpenCode Connections</h2>
                  <p className="text-xs text-white/40">
                    Manage backend connections (Claude Code, etc.)
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowNewConnection(true)}
                className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-1.5 text-xs text-white/70 hover:bg-white/[0.04] transition-colors"
              >
                <Plus className="h-3 w-3" />
                Add
              </button>
            </div>

            {/* Connection List */}
            <div className="space-y-2">
              {connectionsLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Loader className="h-5 w-5 animate-spin text-white/40" />
                </div>
              ) : connections.length === 0 ? (
                <p className="text-center text-xs text-white/40 py-6">
                  No connections configured.
                  <br />
                  Add one to connect to OpenCode backends like Claude Code.
                </p>
              ) : (
                connections.map((conn) => (
                  <div
                    key={conn.id}
                    className={cn(
                      'rounded-lg border p-3 transition-colors',
                      conn.is_default
                        ? 'border-violet-500/30 bg-violet-500/5'
                        : 'border-white/[0.06] bg-white/[0.01]'
                    )}
                  >
                    {editingConnection === conn.id ? (
                      // Edit mode
                      <div className="space-y-3">
                        <input
                          type="text"
                          value={editForm.name ?? ''}
                          onChange={(e) =>
                            setEditForm({ ...editForm, name: e.target.value })
                          }
                          placeholder="Name"
                          className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white focus:outline-none focus:border-violet-500/50"
                        />
                        <input
                          type="text"
                          value={editForm.base_url ?? ''}
                          onChange={(e) =>
                            setEditForm({ ...editForm, base_url: e.target.value })
                          }
                          placeholder="Base URL"
                          className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white focus:outline-none focus:border-violet-500/50"
                        />
                        <input
                          type="text"
                          value={editForm.agent ?? ''}
                          onChange={(e) =>
                            setEditForm({
                              ...editForm,
                              agent: e.target.value || null,
                            })
                          }
                          placeholder="Agent (optional)"
                          className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white focus:outline-none focus:border-violet-500/50"
                        />
                        <div className="flex items-center gap-4">
                          <label className="flex items-center gap-2 text-xs text-white/60">
                            <input
                              type="checkbox"
                              checked={editForm.permissive ?? true}
                              onChange={(e) =>
                                setEditForm({ ...editForm, permissive: e.target.checked })
                              }
                              className="rounded border-white/20"
                            />
                            Permissive
                          </label>
                          <label className="flex items-center gap-2 text-xs text-white/60">
                            <input
                              type="checkbox"
                              checked={editForm.enabled ?? true}
                              onChange={(e) =>
                                setEditForm({ ...editForm, enabled: e.target.checked })
                              }
                              className="rounded border-white/20"
                            />
                            Enabled
                          </label>
                        </div>
                        <div className="flex items-center gap-2 pt-1">
                          <button
                            onClick={handleSaveEdit}
                            className="flex items-center gap-1.5 rounded-lg bg-violet-500 px-3 py-1.5 text-xs text-white hover:bg-violet-600 transition-colors"
                          >
                            <Check className="h-3 w-3" />
                            Save
                          </button>
                          <button
                            onClick={handleCancelEdit}
                            className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] px-3 py-1.5 text-xs text-white/70 hover:bg-white/[0.04] transition-colors"
                          >
                            <X className="h-3 w-3" />
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      // View mode
                      <div>
                        <div className="flex items-start justify-between">
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <h3 className="text-sm font-medium text-white truncate">
                                {conn.name}
                              </h3>
                              {conn.is_default && (
                                <span className="flex items-center gap-1 rounded-full bg-violet-500/20 px-2 py-0.5 text-[10px] font-medium text-violet-400">
                                  <Star className="h-2.5 w-2.5" />
                                  Default
                                </span>
                              )}
                              {!conn.enabled && (
                                <span className="rounded-full bg-white/10 px-2 py-0.5 text-[10px] text-white/40">
                                  Disabled
                                </span>
                              )}
                            </div>
                            <p className="text-xs text-white/40 truncate mt-0.5">
                              {conn.base_url}
                            </p>
                            {conn.agent && (
                              <p className="text-xs text-white/30 mt-0.5">
                                Agent: {conn.agent}
                              </p>
                            )}
                          </div>
                        </div>
                        <div className="flex items-center gap-2 mt-3">
                          <button
                            onClick={() => handleTestConnection(conn.id)}
                            disabled={testingConnectionId === conn.id}
                            className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] bg-white/[0.02] px-2.5 py-1 text-xs text-white/60 hover:bg-white/[0.04] transition-colors disabled:opacity-50"
                          >
                            {testingConnectionId === conn.id ? (
                              <Loader className="h-3 w-3 animate-spin" />
                            ) : (
                              <ExternalLink className="h-3 w-3" />
                            )}
                            Test
                          </button>
                          {!conn.is_default && (
                            <button
                              onClick={() => handleSetDefault(conn.id)}
                              className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] bg-white/[0.02] px-2.5 py-1 text-xs text-white/60 hover:bg-white/[0.04] transition-colors"
                            >
                              <Star className="h-3 w-3" />
                              Set Default
                            </button>
                          )}
                          <button
                            onClick={() => handleStartEdit(conn)}
                            className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] bg-white/[0.02] px-2.5 py-1 text-xs text-white/60 hover:bg-white/[0.04] transition-colors"
                          >
                            Edit
                          </button>
                          <button
                            onClick={() => handleDeleteConnection(conn.id)}
                            className="flex items-center gap-1.5 rounded-lg border border-red-500/20 bg-red-500/5 px-2.5 py-1 text-xs text-red-400 hover:bg-red-500/10 transition-colors"
                          >
                            <Trash2 className="h-3 w-3" />
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>

            {/* New Connection Form */}
            {showNewConnection && (
              <div className="mt-4 rounded-lg border border-violet-500/30 bg-violet-500/5 p-4">
                <h3 className="text-sm font-medium text-white mb-3">New Connection</h3>
                <div className="space-y-3">
                  <div>
                    <label className="block text-xs font-medium text-white/60 mb-1">
                      Name
                    </label>
                    <input
                      type="text"
                      value={newConnection.name}
                      onChange={(e) =>
                        setNewConnection({ ...newConnection, name: e.target.value })
                      }
                      placeholder="e.g., Claude Code"
                      className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:border-violet-500/50"
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-medium text-white/60 mb-1">
                      Base URL
                    </label>
                    <input
                      type="text"
                      value={newConnection.base_url}
                      onChange={(e) =>
                        setNewConnection({ ...newConnection, base_url: e.target.value })
                      }
                      placeholder="http://127.0.0.1:4096"
                      className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:border-violet-500/50"
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-medium text-white/60 mb-1">
                      Agent (optional)
                    </label>
                    <input
                      type="text"
                      value={newConnection.agent}
                      onChange={(e) =>
                        setNewConnection({ ...newConnection, agent: e.target.value })
                      }
                      placeholder="e.g., build, plan"
                      className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:border-violet-500/50"
                    />
                  </div>
                  <label className="flex items-center gap-2 text-xs text-white/60">
                    <input
                      type="checkbox"
                      checked={newConnection.permissive}
                      onChange={(e) =>
                        setNewConnection({
                          ...newConnection,
                          permissive: e.target.checked,
                        })
                      }
                      className="rounded border-white/20"
                    />
                    Permissive mode (auto-allow all permissions)
                  </label>
                  <div className="flex items-center gap-2 pt-1">
                    <button
                      onClick={handleCreateConnection}
                      disabled={savingConnection}
                      className="flex items-center gap-1.5 rounded-lg bg-violet-500 px-3 py-1.5 text-xs text-white hover:bg-violet-600 transition-colors disabled:opacity-50"
                    >
                      {savingConnection ? (
                        <Loader className="h-3 w-3 animate-spin" />
                      ) : (
                        <Plus className="h-3 w-3" />
                      )}
                      Create Connection
                    </button>
                    <button
                      onClick={() => setShowNewConnection(false)}
                      className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] px-3 py-1.5 text-xs text-white/70 hover:bg-white/[0.04] transition-colors"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Configuration Library */}
          <div className="rounded-xl bg-white/[0.02] border border-white/[0.04] p-5">
            <div className="flex items-center gap-3 mb-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-500/10">
                <GitBranch className="h-5 w-5 text-indigo-400" />
              </div>
              <div>
                <h2 className="text-sm font-medium text-white">Configuration Library</h2>
                <p className="text-xs text-white/40">
                  Git repo for MCPs, skills, and commands
                </p>
              </div>
            </div>

            <div>
              <label className="block text-xs font-medium text-white/60 mb-1.5">
                Library Repo (optional)
              </label>
              <input
                type="text"
                value={libraryRepo}
                onChange={(e) => {
                  setLibraryRepo(e.target.value);
                  validateRepo(e.target.value);
                }}
                placeholder="https://github.com/your/library.git"
                className={cn(
                  'w-full rounded-lg border bg-white/[0.02] px-3 py-2.5 text-sm text-white placeholder-white/30 focus:outline-none transition-colors',
                  repoError
                    ? 'border-red-500/50 focus:border-red-500/50'
                    : 'border-white/[0.06] focus:border-indigo-500/50'
                )}
              />
              {repoError ? (
                <p className="mt-1.5 text-xs text-red-400">{repoError}</p>
              ) : (
                <p className="mt-1.5 text-xs text-white/30">
                  Leave blank to disable library features.
                </p>
              )}
            </div>
          </div>

          {/* Save Button */}
          <button
            onClick={handleSave}
            disabled={!!urlError || !!repoError}
            className={cn(
              'w-full flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-medium text-white transition-colors',
              urlError || repoError
                ? 'bg-white/10 cursor-not-allowed opacity-50'
                : 'bg-indigo-500 hover:bg-indigo-600'
            )}
          >
            <Save className="h-4 w-4" />
            Save Settings
            <span className="text-xs text-white/50 ml-1">(⌘S)</span>
          </button>
        </div>
      </div>
    </div>
  );
}
