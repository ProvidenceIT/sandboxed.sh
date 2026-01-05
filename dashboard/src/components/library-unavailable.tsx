'use client';

import { useState } from 'react';
import { GitBranch, ArrowRight, Loader } from 'lucide-react';
import { readSavedSettings, writeSavedSettings } from '@/lib/settings';
import { cn } from '@/lib/utils';

type LibraryUnavailableProps = {
  message?: string | null;
  onConfigured?: () => void;
};

export function LibraryUnavailable({ message, onConfigured }: LibraryUnavailableProps) {
  const details = message?.trim();
  const showDetails = !!details && details !== "Library not initialized";

  const [repoUrl, setRepoUrl] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = repoUrl.trim();
    if (!trimmed) {
      setError('Please enter a repository URL');
      return;
    }
    if (/\s/.test(trimmed)) {
      setError('Repository URL cannot contain spaces');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      const current = readSavedSettings();
      writeSavedSettings({ ...current, libraryRepo: trimmed });
      // Trigger reload after a brief delay
      setTimeout(() => {
        if (onConfigured) {
          onConfigured();
        } else {
          window.location.reload();
        }
      }, 100);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save');
      setSaving(false);
    }
  };

  return (
    <div className="flex items-center justify-center min-h-[calc(100vh-12rem)]">
      <div className="w-full max-w-md text-center">
        <div className="flex justify-center mb-6">
          <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-indigo-500/10 border border-indigo-500/20">
            <GitBranch className="h-8 w-8 text-indigo-400" />
          </div>
        </div>

        <h2 className="text-lg font-semibold text-white mb-2">
          Configure Library
        </h2>
        <p className="text-sm text-white/50 mb-6">
          Connect a Git repository to store and sync your MCPs, skills, and commands.
        </p>

        <form onSubmit={handleSubmit} className="space-y-3">
          <div className="relative">
            <input
              type="text"
              value={repoUrl}
              onChange={(e) => {
                setRepoUrl(e.target.value);
                setError(null);
              }}
              placeholder="https://github.com/your/library.git"
              className={cn(
                "w-full rounded-xl border bg-white/[0.02] px-4 py-3 text-sm text-white placeholder-white/30 focus:outline-none transition-colors",
                error
                  ? "border-red-500/50 focus:border-red-500/50"
                  : "border-white/[0.08] focus:border-indigo-500/50"
              )}
              disabled={saving}
            />
          </div>

          {error && (
            <p className="text-xs text-red-400 text-left">{error}</p>
          )}

          <button
            type="submit"
            disabled={saving || !repoUrl.trim()}
            className="w-full flex items-center justify-center gap-2 rounded-xl bg-indigo-500 hover:bg-indigo-600 disabled:bg-indigo-500/50 px-4 py-3 text-sm font-medium text-white transition-colors disabled:cursor-not-allowed"
          >
            {saving ? (
              <>
                <Loader className="h-4 w-4 animate-spin" />
                Configuring...
              </>
            ) : (
              <>
                Connect Repository
                <ArrowRight className="h-4 w-4" />
              </>
            )}
          </button>
        </form>

        <p className="mt-6 text-xs text-white/30">
          The repository should be accessible via SSH or HTTPS.
          {' '}
          <a
            href="https://docs.github.com/en/repositories/creating-and-managing-repositories/cloning-a-repository"
            target="_blank"
            rel="noopener noreferrer"
            className="text-indigo-400/70 hover:text-indigo-400 transition-colors"
          >
            Learn more
          </a>
        </p>

        {showDetails && (
          <p className="mt-4 text-[11px] text-white/20">
            Details: {details}
          </p>
        )}
      </div>
    </div>
  );
}
