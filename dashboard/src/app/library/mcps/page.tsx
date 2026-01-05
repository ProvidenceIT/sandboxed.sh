'use client';

import { useEffect, useState } from 'react';
import {
  getLibraryStatus,
  syncLibrary,
  commitLibrary,
  pushLibrary,
  getLibraryMcps,
  saveLibraryMcps,
  type LibraryStatus,
  type McpServerDef,
} from '@/lib/api';
import {
  GitBranch,
  RefreshCw,
  Upload,
  Check,
  AlertCircle,
  Plug,
  Loader,
  Save,
  X,
} from 'lucide-react';
import { cn } from '@/lib/utils';

export default function McpsPage() {
  const [status, setStatus] = useState<LibraryStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [committing, setCommitting] = useState(false);
  const [pushing, setPushing] = useState(false);
  const [commitMessage, setCommitMessage] = useState('');
  const [showCommitDialog, setShowCommitDialog] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [mcps, setMcps] = useState<Record<string, McpServerDef>>({});
  const [mcpJsonContent, setMcpJsonContent] = useState('');
  const [mcpParseError, setMcpParseError] = useState<string | null>(null);
  const [mcpDirty, setMcpDirty] = useState(false);
  const [mcpSaving, setMcpSaving] = useState(false);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const [statusData, mcpsData] = await Promise.all([
        getLibraryStatus(),
        getLibraryMcps(),
      ]);
      setStatus(statusData);
      setMcps(mcpsData);
      setMcpJsonContent(JSON.stringify(mcpsData, null, 2));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load MCPs');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleSync = async () => {
    try {
      setSyncing(true);
      await syncLibrary();
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to sync');
    } finally {
      setSyncing(false);
    }
  };

  const handleCommit = async () => {
    if (!commitMessage.trim()) return;
    try {
      setCommitting(true);
      await commitLibrary(commitMessage);
      setCommitMessage('');
      setShowCommitDialog(false);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to commit');
    } finally {
      setCommitting(false);
    }
  };

  const handlePush = async () => {
    try {
      setPushing(true);
      await pushLibrary();
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to push');
    } finally {
      setPushing(false);
    }
  };

  const handleMcpContentChange = (value: string) => {
    setMcpJsonContent(value);
    setMcpDirty(true);
    setMcpParseError(null);
    try {
      JSON.parse(value);
    } catch (err) {
      setMcpParseError(err instanceof Error ? err.message : 'Invalid JSON');
    }
  };

  const handleMcpSave = async () => {
    if (mcpParseError) return;
    try {
      setMcpSaving(true);
      const parsed = JSON.parse(mcpJsonContent);
      await saveLibraryMcps(parsed);
      setMcps(parsed);
      setMcpDirty(false);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save MCPs');
    } finally {
      setMcpSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader className="h-8 w-8 animate-spin text-white/40" />
      </div>
    );
  }

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="mb-8">
        <div className="flex items-center gap-4 mb-2">
          <div className="p-2.5 rounded-lg bg-indigo-500/10">
            <Plug className="h-6 w-6 text-indigo-400" />
          </div>
          <h1 className="text-2xl font-semibold text-white">MCP Servers</h1>
        </div>
        <p className="text-white/50">Configure Model Context Protocol servers.</p>
      </div>

      {error && (
        <div className="mb-6 p-4 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 flex items-center gap-2">
          <AlertCircle className="h-4 w-4 flex-shrink-0" />
          {error}
          <button onClick={() => setError(null)} className="ml-auto">
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      {/* Git Status Bar */}
      {status && (
        <div className="mb-6 p-4 rounded-xl bg-white/[0.02] border border-white/[0.06]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <GitBranch className="h-4 w-4 text-white/40" />
                <span className="text-sm font-medium text-white">{status.branch}</span>
              </div>
              <div className="flex items-center gap-2">
                {status.clean ? (
                  <span className="flex items-center gap-1 text-xs text-emerald-400">
                    <Check className="h-3 w-3" />
                    Clean
                  </span>
                ) : (
                  <span className="flex items-center gap-1 text-xs text-amber-400">
                    <AlertCircle className="h-3 w-3" />
                    {status.modified_files.length} modified
                  </span>
                )}
              </div>
              {(status.ahead > 0 || status.behind > 0) && (
                <div className="text-xs text-white/40">
                  {status.ahead > 0 && <span className="text-emerald-400">+{status.ahead}</span>}
                  {status.ahead > 0 && status.behind > 0 && ' / '}
                  {status.behind > 0 && <span className="text-amber-400">-{status.behind}</span>}
                </div>
              )}
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={handleSync}
                disabled={syncing}
                className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-white/70 hover:text-white bg-white/[0.04] hover:bg-white/[0.08] rounded-lg transition-colors disabled:opacity-50"
              >
                <RefreshCw className={cn('h-3 w-3', syncing && 'animate-spin')} />
                Sync
              </button>
              {!status.clean && (
                <button
                  onClick={() => setShowCommitDialog(true)}
                  disabled={committing}
                  className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-white/70 hover:text-white bg-white/[0.04] hover:bg-white/[0.08] rounded-lg transition-colors disabled:opacity-50"
                >
                  <Check className="h-3 w-3" />
                  Commit
                </button>
              )}
              {status.ahead > 0 && (
                <button
                  onClick={handlePush}
                  disabled={pushing}
                  className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-emerald-400 hover:text-emerald-300 bg-emerald-500/10 hover:bg-emerald-500/20 rounded-lg transition-colors disabled:opacity-50"
                >
                  <Upload className={cn('h-3 w-3', pushing && 'animate-pulse')} />
                  Push
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* MCP Editor */}
      <div className="rounded-xl bg-white/[0.02] border border-white/[0.06] overflow-hidden">
        <div className="p-4 border-b border-white/[0.06] flex items-center justify-between">
          <span className="text-xs text-white/40">mcp/servers.json</span>
          <div className="flex items-center gap-2">
            {mcpDirty && <span className="text-xs text-amber-400">Unsaved</span>}
            <button
              onClick={handleMcpSave}
              disabled={mcpSaving || !!mcpParseError || !mcpDirty}
              className={cn(
                'flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors',
                mcpDirty && !mcpParseError
                  ? 'text-white bg-indigo-500 hover:bg-indigo-600'
                  : 'text-white/40 bg-white/[0.04]'
              )}
            >
              <Save className={cn('h-3 w-3', mcpSaving && 'animate-pulse')} />
              Save
            </button>
          </div>
        </div>
        {mcpParseError && (
          <div className="mx-4 mt-4 p-2 rounded-lg bg-amber-500/10 text-amber-400 text-xs flex items-center gap-2">
            <AlertCircle className="h-3 w-3" />
            {mcpParseError}
          </div>
        )}
        <div className="p-4">
          <textarea
            value={mcpJsonContent}
            onChange={(e) => handleMcpContentChange(e.target.value)}
            className="w-full h-96 font-mono text-sm bg-[#0d0d0e] border border-white/[0.06] rounded-lg p-4 text-white/90 resize-none focus:outline-none focus:border-indigo-500/50"
            spellCheck={false}
          />
        </div>
      </div>

      {/* Commit Dialog */}
      {showCommitDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-full max-w-md p-6 rounded-xl bg-[#1a1a1c] border border-white/[0.06]">
            <h3 className="text-lg font-medium text-white mb-4">Commit Changes</h3>
            <input
              type="text"
              placeholder="Commit message..."
              value={commitMessage}
              onChange={(e) => setCommitMessage(e.target.value)}
              className="w-full px-4 py-2 rounded-lg bg-white/[0.04] border border-white/[0.08] text-white placeholder:text-white/30 focus:outline-none focus:border-indigo-500/50 mb-4"
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowCommitDialog(false)}
                className="px-4 py-2 text-sm text-white/60 hover:text-white"
              >
                Cancel
              </button>
              <button
                onClick={handleCommit}
                disabled={!commitMessage.trim() || committing}
                className="px-4 py-2 text-sm font-medium text-white bg-indigo-500 hover:bg-indigo-600 rounded-lg disabled:opacity-50"
              >
                {committing ? 'Committing...' : 'Commit'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
