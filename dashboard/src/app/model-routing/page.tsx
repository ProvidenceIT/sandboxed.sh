'use client';

import { useState } from 'react';
import useSWR from 'swr';
import { toast } from '@/components/toast';
import {
  listModelChains,
  createModelChain,
  updateModelChain,
  deleteModelChain,
  resolveModelChain,
  listAccountHealth,
  clearAccountCooldown,
  type ModelChain,
  type ChainEntry,
  type ResolvedEntry,
  type AccountHealthSnapshot,
} from '@/lib/api/model-routing';
import {
  GitBranch,
  Plus,
  Trash2,
  Star,
  Loader,
  ChevronDown,
  ChevronRight,
  Heart,
  AlertTriangle,
  Clock,
  RotateCcw,
  GripVertical,
  ArrowDown,
  ArrowUp,
} from 'lucide-react';
import { cn } from '@/lib/utils';

// ─────────────────────────────────────────────────────────────────────────────
// Chain Entry Editor
// ─────────────────────────────────────────────────────────────────────────────

function EntryEditor({
  entries,
  onChange,
}: {
  entries: ChainEntry[];
  onChange: (entries: ChainEntry[]) => void;
}) {
  const addEntry = () => {
    onChange([...entries, { provider_id: '', model_id: '' }]);
  };

  const removeEntry = (index: number) => {
    onChange(entries.filter((_, i) => i !== index));
  };

  const updateEntry = (index: number, field: keyof ChainEntry, value: string) => {
    const updated = entries.map((e, i) =>
      i === index ? { ...e, [field]: value } : e
    );
    onChange(updated);
  };

  const moveEntry = (index: number, direction: 'up' | 'down') => {
    const newIndex = direction === 'up' ? index - 1 : index + 1;
    if (newIndex < 0 || newIndex >= entries.length) return;
    const updated = [...entries];
    [updated[index], updated[newIndex]] = [updated[newIndex], updated[index]];
    onChange(updated);
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-white/40">
          Fallback chain (tried in order)
        </span>
        <button
          onClick={addEntry}
          className="flex items-center gap-1 text-xs text-indigo-400 hover:text-indigo-300 transition-colors cursor-pointer"
        >
          <Plus className="h-3 w-3" />
          Add entry
        </button>
      </div>
      {entries.map((entry, i) => (
        <div
          key={i}
          className="flex items-center gap-2 rounded-lg border border-white/[0.06] bg-white/[0.01] px-2 py-1.5"
        >
          <GripVertical className="h-3.5 w-3.5 text-white/20 flex-shrink-0" />
          <span className="text-[10px] text-white/30 w-4 flex-shrink-0">
            {i + 1}.
          </span>
          <input
            type="text"
            value={entry.provider_id}
            onChange={(e) => updateEntry(i, 'provider_id', e.target.value)}
            placeholder="provider (e.g. zai)"
            className="flex-1 min-w-0 rounded border border-white/[0.06] bg-white/[0.02] px-2 py-1 text-xs text-white focus:outline-none focus:border-indigo-500/50"
          />
          <span className="text-white/20">/</span>
          <input
            type="text"
            value={entry.model_id}
            onChange={(e) => updateEntry(i, 'model_id', e.target.value)}
            placeholder="model (e.g. glm-4-plus)"
            className="flex-1 min-w-0 rounded border border-white/[0.06] bg-white/[0.02] px-2 py-1 text-xs text-white focus:outline-none focus:border-indigo-500/50"
          />
          <div className="flex items-center gap-0.5 flex-shrink-0">
            <button
              onClick={() => moveEntry(i, 'up')}
              disabled={i === 0}
              className="p-0.5 text-white/20 hover:text-white/60 disabled:opacity-20 cursor-pointer"
            >
              <ArrowUp className="h-3 w-3" />
            </button>
            <button
              onClick={() => moveEntry(i, 'down')}
              disabled={i === entries.length - 1}
              className="p-0.5 text-white/20 hover:text-white/60 disabled:opacity-20 cursor-pointer"
            >
              <ArrowDown className="h-3 w-3" />
            </button>
            <button
              onClick={() => removeEntry(i)}
              className="p-0.5 text-white/20 hover:text-red-400 cursor-pointer"
            >
              <Trash2 className="h-3 w-3" />
            </button>
          </div>
        </div>
      ))}
      {entries.length === 0 && (
        <p className="text-xs text-white/30 text-center py-3">
          No entries yet. Add provider/model pairs to define the fallback chain.
        </p>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Chain Card
// ─────────────────────────────────────────────────────────────────────────────

function ChainCard({
  chain,
  onUpdate,
  onDelete,
  onSetDefault,
}: {
  chain: ModelChain;
  onUpdate: (id: string, data: { name?: string; entries?: ChainEntry[]; is_default?: boolean }) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onSetDefault: (id: string) => Promise<void>;
}) {
  const [expanded, setExpanded] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState(chain.name);
  const [editEntries, setEditEntries] = useState<ChainEntry[]>(chain.entries);
  const [resolved, setResolved] = useState<ResolvedEntry[] | null>(null);
  const [loadingResolve, setLoadingResolve] = useState(false);

  const handleResolve = async () => {
    setLoadingResolve(true);
    try {
      const entries = await resolveModelChain(chain.id);
      setResolved(entries);
    } catch (err) {
      toast.error(`Failed to resolve: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setLoadingResolve(false);
    }
  };

  const handleSave = async () => {
    const validEntries = editEntries.filter(
      (e) => e.provider_id.trim() && e.model_id.trim()
    );
    if (validEntries.length === 0) {
      toast.error('At least one valid entry is required');
      return;
    }
    try {
      await onUpdate(chain.id, { name: editName, entries: validEntries });
      setEditing(false);
    } catch {
      // onUpdate already shows a toast; stay in edit mode so changes aren't lost
    }
  };

  const handleStartEdit = () => {
    setEditName(chain.name);
    setEditEntries([...chain.entries]);
    setEditing(true);
    setExpanded(true);
  };

  return (
    <div className="rounded-lg border border-white/[0.06] bg-white/[0.01] hover:bg-white/[0.02] transition-colors">
      <div
        className="flex items-center gap-3 px-3 py-2.5 cursor-pointer"
        onClick={() => !editing && setExpanded(!expanded)}
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 text-white/30 flex-shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-white/30 flex-shrink-0" />
        )}
        <GitBranch className="h-4 w-4 text-indigo-400 flex-shrink-0" />
        <div className="flex-1 min-w-0">
          <span className="text-sm text-white/80">{chain.name}</span>
          <span className="ml-2 text-xs text-white/30 font-mono">{chain.id}</span>
        </div>
        <span className="text-[10px] text-white/30">
          {chain.entries.length} {chain.entries.length === 1 ? 'entry' : 'entries'}
        </span>
        {chain.is_default && (
          <Star className="h-3 w-3 text-indigo-400 fill-indigo-400 flex-shrink-0" />
        )}
        <div
          className="flex items-center gap-0.5"
          onClick={(e) => e.stopPropagation()}
        >
          {!chain.is_default && (
            <button
              onClick={() => onSetDefault(chain.id)}
              className="p-1.5 rounded-md text-white/20 hover:text-indigo-400 hover:bg-white/[0.04] transition-colors cursor-pointer"
              title="Set as default"
            >
              <Star className="h-3.5 w-3.5" />
            </button>
          )}
          <button
            onClick={handleStartEdit}
            className="p-1.5 rounded-md text-white/20 hover:text-white/60 hover:bg-white/[0.04] transition-colors cursor-pointer"
            title="Edit"
          >
            <GitBranch className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={() => onDelete(chain.id)}
            className="p-1.5 rounded-md text-white/20 hover:text-red-400 hover:bg-white/[0.04] transition-colors cursor-pointer"
            title="Delete"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {expanded && (
        <div className="border-t border-white/[0.04] px-3 py-3 space-y-3">
          {editing ? (
            <>
              <div>
                <label className="text-xs text-white/40 mb-1 block">Chain name</label>
                <input
                  type="text"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500/50"
                />
              </div>
              <EntryEditor entries={editEntries} onChange={setEditEntries} />
              <div className="flex items-center justify-end gap-2 pt-1">
                <button
                  onClick={() => setEditing(false)}
                  className="rounded-lg px-3 py-1.5 text-xs text-white/60 hover:text-white/80 transition-colors cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSave}
                  className="rounded-lg bg-indigo-500 px-3 py-1.5 text-xs text-white hover:bg-indigo-600 transition-colors cursor-pointer"
                >
                  Save
                </button>
              </div>
            </>
          ) : (
            <>
              {/* Entries list */}
              <div className="space-y-1">
                {chain.entries.map((entry, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 text-xs"
                  >
                    <span className="text-white/20 w-4">{i + 1}.</span>
                    <span className="text-white/60 font-mono">
                      {entry.provider_id}/{entry.model_id}
                    </span>
                  </div>
                ))}
              </div>

              {/* Resolve button */}
              <div className="pt-1">
                <button
                  onClick={handleResolve}
                  disabled={loadingResolve}
                  className="text-xs text-indigo-400 hover:text-indigo-300 transition-colors cursor-pointer disabled:opacity-50"
                >
                  {loadingResolve ? (
                    <span className="flex items-center gap-1">
                      <Loader className="h-3 w-3 animate-spin" />
                      Resolving...
                    </span>
                  ) : (
                    'Test chain resolution'
                  )}
                </button>
                {resolved && (
                  <div className="mt-2 space-y-1 rounded-lg bg-white/[0.02] border border-white/[0.04] p-2">
                    <span className="text-[10px] text-white/30 uppercase tracking-wider">
                      Resolved entries ({resolved.length})
                    </span>
                    {resolved.length === 0 ? (
                      <p className="text-xs text-amber-400">
                        No healthy accounts available for this chain
                      </p>
                    ) : (
                      resolved.map((r, i) => (
                        <div key={i} className="flex items-center gap-2 text-xs">
                          <span className="text-white/20 w-4">{i + 1}.</span>
                          <span className="text-white/60 font-mono">
                            {r.provider_id}/{r.model_id}
                          </span>
                          <span className="text-white/20 font-mono text-[10px]">
                            {r.account_id.slice(0, 8)}
                          </span>
                          <span className={cn(
                            'h-1.5 w-1.5 rounded-full',
                            r.has_api_key ? 'bg-emerald-400' : 'bg-red-400'
                          )} />
                        </div>
                      ))
                    )}
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Health Dashboard
// ─────────────────────────────────────────────────────────────────────────────

function HealthDashboard({
  health,
  onClear,
  isLoading,
}: {
  health: AccountHealthSnapshot[];
  onClear: (accountId: string) => Promise<void>;
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader className="h-5 w-5 animate-spin text-white/40" />
      </div>
    );
  }

  if (health.length === 0) {
    return (
      <div className="text-center py-6">
        <p className="text-xs text-white/30">
          No health data yet. Health tracking begins when the proxy handles requests.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {health.map((h) => (
        <div
          key={h.account_id}
          className="flex items-center gap-3 rounded-lg border border-white/[0.06] bg-white/[0.01] px-3 py-2"
        >
          <span
            className={cn(
              'h-2 w-2 rounded-full flex-shrink-0',
              h.is_healthy ? 'bg-emerald-400' : 'bg-red-400'
            )}
          />
          <span className="text-xs text-white/50 font-mono flex-shrink-0">
            {h.account_id.slice(0, 8)}...
          </span>
          <div className="flex-1 flex items-center gap-3 text-[10px] text-white/30">
            <span>{h.total_requests} req</span>
            <span className="text-emerald-400/60">{h.total_successes} ok</span>
            {h.total_rate_limits > 0 && (
              <span className="text-amber-400/60">{h.total_rate_limits} rate-limited</span>
            )}
            {h.total_errors > 0 && (
              <span className="text-red-400/60">{h.total_errors} errors</span>
            )}
          </div>
          {!h.is_healthy && (
            <div className="flex items-center gap-2 flex-shrink-0">
              {h.cooldown_remaining_secs != null && (
                <span className="flex items-center gap-1 text-[10px] text-amber-400">
                  <Clock className="h-3 w-3" />
                  {Math.ceil(h.cooldown_remaining_secs)}s
                </span>
              )}
              {h.last_failure_reason && (
                <span className="text-[10px] text-red-400/60">
                  {h.last_failure_reason}
                </span>
              )}
              <button
                onClick={() => onClear(h.account_id)}
                className="p-1 rounded text-amber-400 hover:bg-white/[0.04] transition-colors cursor-pointer"
                title="Clear cooldown"
              >
                <RotateCcw className="h-3 w-3" />
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Page
// ─────────────────────────────────────────────────────────────────────────────

export default function ModelRoutingPage() {
  const [showCreate, setShowCreate] = useState(false);
  const [createForm, setCreateForm] = useState({
    id: '',
    name: '',
    entries: [{ provider_id: '', model_id: '' }] as ChainEntry[],
    is_default: false,
  });

  const {
    data: chains = [],
    isLoading: chainsLoading,
    mutate: mutateChains,
  } = useSWR('model-chains', listModelChains, { revalidateOnFocus: false });

  const {
    data: health = [],
    isLoading: healthLoading,
    mutate: mutateHealth,
  } = useSWR('account-health', listAccountHealth, {
    revalidateOnFocus: false,
    refreshInterval: 10000, // Poll health every 10s
  });

  const handleCreate = async () => {
    if (!createForm.id.trim() || !createForm.name.trim()) {
      toast.error('Chain ID and name are required');
      return;
    }
    const validEntries = createForm.entries.filter(
      (e) => e.provider_id.trim() && e.model_id.trim()
    );
    if (validEntries.length === 0) {
      toast.error('At least one valid entry is required');
      return;
    }
    try {
      await createModelChain({
        id: createForm.id.trim(),
        name: createForm.name.trim(),
        entries: validEntries,
        is_default: createForm.is_default,
      });
      toast.success('Chain created');
      setShowCreate(false);
      setCreateForm({
        id: '',
        name: '',
        entries: [{ provider_id: '', model_id: '' }],
        is_default: false,
      });
      mutateChains();
    } catch (err) {
      toast.error(`Failed to create: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleUpdate = async (
    id: string,
    data: { name?: string; entries?: ChainEntry[]; is_default?: boolean }
  ) => {
    try {
      await updateModelChain(id, data);
      toast.success('Chain updated');
      mutateChains();
    } catch (err) {
      toast.error(`Failed to update: ${err instanceof Error ? err.message : 'Unknown error'}`);
      throw err;
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteModelChain(id);
      toast.success('Chain deleted');
      mutateChains();
    } catch (err) {
      toast.error(`Failed to delete: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await updateModelChain(id, { is_default: true });
      toast.success('Default chain updated');
      mutateChains();
    } catch (err) {
      toast.error(`Failed to set default: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const handleClearCooldown = async (accountId: string) => {
    try {
      await clearAccountCooldown(accountId);
      toast.success('Cooldown cleared');
      mutateHealth();
    } catch (err) {
      toast.error(`Failed to clear: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  return (
    <div className="flex-1 flex flex-col items-center p-6 overflow-auto">
      <div className="w-full max-w-2xl">
        <div className="mb-8">
          <h1 className="text-xl font-semibold text-white">Model Routing</h1>
          <p className="mt-1 text-sm text-white/50">
            Configure fallback chains and monitor provider health
          </p>
        </div>

        {/* ── Chains Section ── */}
        <div className="rounded-xl bg-white/[0.02] border border-white/[0.04] p-5 mb-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-500/10">
                <GitBranch className="h-5 w-5 text-indigo-400" />
              </div>
              <div>
                <h2 className="text-sm font-medium text-white">Fallback Chains</h2>
                <p className="text-xs text-white/40">
                  Define provider/model fallback order for the proxy
                </p>
              </div>
            </div>
            <button
              onClick={() => setShowCreate(!showCreate)}
              className="flex items-center gap-1.5 rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-1.5 text-xs text-white/70 hover:bg-white/[0.04] transition-colors cursor-pointer"
            >
              <Plus className="h-3 w-3" />
              New Chain
            </button>
          </div>

          {/* Create form */}
          {showCreate && (
            <div className="mb-4 rounded-lg border border-indigo-500/20 bg-indigo-500/[0.03] p-4 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs text-white/40 mb-1 block">Chain ID</label>
                  <input
                    type="text"
                    value={createForm.id}
                    onChange={(e) =>
                      setCreateForm({ ...createForm, id: e.target.value })
                    }
                    placeholder="e.g. builtin/fast"
                    className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500/50"
                  />
                </div>
                <div>
                  <label className="text-xs text-white/40 mb-1 block">Display name</label>
                  <input
                    type="text"
                    value={createForm.name}
                    onChange={(e) =>
                      setCreateForm({ ...createForm, name: e.target.value })
                    }
                    placeholder="e.g. Fast Chain"
                    className="w-full rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-white focus:outline-none focus:border-indigo-500/50"
                  />
                </div>
              </div>
              <EntryEditor
                entries={createForm.entries}
                onChange={(entries) =>
                  setCreateForm({ ...createForm, entries })
                }
              />
              <div className="flex items-center justify-between pt-1">
                <label className="flex items-center gap-2 text-xs text-white/60 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={createForm.is_default}
                    onChange={(e) =>
                      setCreateForm({ ...createForm, is_default: e.target.checked })
                    }
                    className="rounded border-white/20 cursor-pointer"
                  />
                  Set as default chain
                </label>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => setShowCreate(false)}
                    className="rounded-lg px-3 py-1.5 text-xs text-white/60 hover:text-white/80 transition-colors cursor-pointer"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleCreate}
                    className="rounded-lg bg-indigo-500 px-3 py-1.5 text-xs text-white hover:bg-indigo-600 transition-colors cursor-pointer"
                  >
                    Create
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Chain list */}
          <div className="space-y-2">
            {chainsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader className="h-5 w-5 animate-spin text-white/40" />
              </div>
            ) : chains.length === 0 ? (
              <div className="text-center py-8">
                <div className="flex justify-center mb-3">
                  <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-white/[0.04]">
                    <GitBranch className="h-6 w-6 text-white/30" />
                  </div>
                </div>
                <p className="text-sm text-white/50 mb-1">No chains configured</p>
                <p className="text-xs text-white/30">
                  The default builtin/smart chain is created automatically on first mission
                </p>
              </div>
            ) : (
              chains.map((chain) => (
                <ChainCard
                  key={chain.id}
                  chain={chain}
                  onUpdate={handleUpdate}
                  onDelete={handleDelete}
                  onSetDefault={handleSetDefault}
                />
              ))
            )}
          </div>
        </div>

        {/* ── Health Section ── */}
        <div className="rounded-xl bg-white/[0.02] border border-white/[0.04] p-5">
          <div className="flex items-center gap-3 mb-4">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500/10">
              <Heart className="h-5 w-5 text-emerald-400" />
            </div>
            <div>
              <h2 className="text-sm font-medium text-white">Provider Health</h2>
              <p className="text-xs text-white/40">
                Per-account health status and cooldown tracking
              </p>
            </div>
            {health.some((h) => !h.is_healthy) && (
              <div className="ml-auto flex items-center gap-1.5">
                <AlertTriangle className="h-3.5 w-3.5 text-amber-400" />
                <span className="text-xs text-amber-400">
                  {health.filter((h) => !h.is_healthy).length} in cooldown
                </span>
              </div>
            )}
          </div>

          <HealthDashboard
            health={health}
            onClear={handleClearCooldown}
            isLoading={healthLoading}
          />
        </div>
      </div>
    </div>
  );
}
