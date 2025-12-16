'use client';

import { useEffect, useState, useRef } from 'react';
import Link from 'next/link';
import { cn } from '@/lib/utils';
import { listTasks, listRuns, TaskState, Run } from '@/lib/api';
import {
  CheckCircle,
  XCircle,
  Clock,
  Loader,
  Ban,
  ArrowRight,
  Search,
  Filter,
  MessageSquare,
} from 'lucide-react';

const statusIcons = {
  pending: Clock,
  running: Loader,
  completed: CheckCircle,
  failed: XCircle,
  cancelled: Ban,
};

const statusColors = {
  pending: 'text-[var(--warning)] bg-[var(--warning)]/10',
  running: 'text-[var(--accent)] bg-[var(--accent)]/10',
  completed: 'text-[var(--success)] bg-[var(--success)]/10',
  failed: 'text-[var(--error)] bg-[var(--error)]/10',
  cancelled: 'text-[var(--foreground-muted)] bg-[var(--foreground-muted)]/10',
};

export default function HistoryPage() {
  const [tasks, setTasks] = useState<TaskState[]>([]);
  const [runs, setRuns] = useState<Run[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string>('all');
  const [search, setSearch] = useState('');
  const fetchedRef = useRef(false);

  useEffect(() => {
    // Prevent double-fetch in React strict mode
    if (fetchedRef.current) return;
    fetchedRef.current = true;
    
    const fetchData = async () => {
      try {
        const [tasksData, runsData] = await Promise.all([
          listTasks().catch(() => []),
          listRuns().catch(() => ({ runs: [] })),
        ]);
        setTasks(tasksData);
        setRuns(runsData.runs || []);
      } catch (error) {
        console.error('Failed to fetch data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  const filteredTasks = tasks.filter((task) => {
    if (filter !== 'all' && task.status !== filter) return false;
    if (search && !task.task.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const filteredRuns = runs.filter((run) => {
    if (filter !== 'all' && run.status !== filter) return false;
    if (search && !run.input_text.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const hasData = filteredTasks.length > 0 || filteredRuns.length > 0;

  return (
    <div className="p-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight text-[var(--foreground)]">
          History
        </h1>
        <p className="mt-1 text-sm text-[var(--foreground-muted)]">
          View all past and current tasks
        </p>
      </div>

      {/* Filters */}
      <div className="mb-6 flex items-center gap-4">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--foreground-muted)]" />
          <input
            type="text"
            placeholder="Search tasks..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full rounded-md border border-[var(--border)] bg-[var(--background-secondary)]/60 py-2 pl-10 pr-4 text-sm text-[var(--foreground)] placeholder-[var(--foreground-muted)] focus:border-[var(--accent)] focus:outline-none focus-visible:!border-[var(--border)]"
          />
        </div>

        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4 text-[var(--foreground-muted)]" />
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="rounded-md border border-[var(--border)] bg-[var(--background-secondary)]/60 px-3 py-2 text-sm text-[var(--foreground)] focus:border-[var(--accent)] focus:outline-none focus-visible:!border-[var(--border)]"
          >
            <option value="all">All Status</option>
            <option value="running">Running</option>
            <option value="completed">Completed</option>
            <option value="failed">Failed</option>
            <option value="pending">Pending</option>
            <option value="cancelled">Cancelled</option>
          </select>
        </div>
      </div>

      {/* Tasks table */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader className="h-8 w-8 animate-spin text-[var(--accent)]" />
        </div>
      ) : !hasData ? (
        <div className="panel rounded-lg p-12 text-center">
          <MessageSquare className="mx-auto h-12 w-12 text-[var(--foreground-muted)]" />
          <p className="mt-4 text-[var(--foreground)]">No history yet</p>
          <p className="mt-2 text-sm text-[var(--foreground-muted)]">
            Start a conversation in the{' '}
            <Link href="/control" className="text-[var(--accent)] hover:underline">
              Control
            </Link>{' '}
            page to interact with the agent.
          </p>
          <p className="mt-1 text-xs text-[var(--foreground-muted)]">
            Note: Control conversations are not persisted here. Create tasks via the API to see them in history.
          </p>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Active Tasks (in-memory) */}
          {filteredTasks.length > 0 && (
            <div>
              <h2 className="mb-3 text-sm font-medium text-[var(--foreground-muted)]">
                Active Tasks ({filteredTasks.length})
              </h2>
              <div className="panel rounded-lg overflow-hidden">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-[var(--border)]">
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Status
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Task
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Model
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Iterations
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Actions
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-[var(--border)]">
                    {filteredTasks.map((task) => {
                      const Icon = statusIcons[task.status];
                      return (
                        <tr
                          key={task.id}
                          className="hover:bg-[var(--background-tertiary)] transition-colors"
                        >
                          <td className="px-4 py-4">
                            <span
                              className={cn(
                                'inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium',
                                statusColors[task.status]
                              )}
                            >
                              <Icon
                                className={cn(
                                  'h-3 w-3',
                                  task.status === 'running' && 'animate-spin'
                                )}
                              />
                              {task.status}
                            </span>
                          </td>
                          <td className="px-4 py-4">
                            <p className="max-w-md truncate text-sm text-[var(--foreground)]">
                              {task.task}
                            </p>
                          </td>
                          <td className="px-4 py-4">
                            <span className="text-sm text-[var(--foreground-muted)]">{task.model}</span>
                          </td>
                          <td className="px-4 py-4">
                            <span className="text-sm text-[var(--foreground)]">{task.iterations}</span>
                          </td>
                          <td className="px-4 py-4">
                            <Link
                              href={`/control?task=${task.id}`}
                              className="inline-flex items-center gap-1 text-sm text-[var(--accent)] hover:underline"
                            >
                              View <ArrowRight className="h-3 w-3" />
                            </Link>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Archived Runs (from memory/Supabase) */}
          {filteredRuns.length > 0 && (
            <div>
              <h2 className="mb-3 text-sm font-medium text-[var(--foreground-muted)]">
                Archived Runs ({filteredRuns.length})
              </h2>
              <div className="panel rounded-lg overflow-hidden">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-[var(--border)]">
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Status
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Input
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Created
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]">
                        Cost
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-[var(--border)]">
                    {filteredRuns.map((run) => {
                      const status = run.status as keyof typeof statusIcons;
                      const Icon = statusIcons[status] || Clock;
                      const colorClass = statusColors[status] || statusColors.pending;
                      return (
                        <tr
                          key={run.id}
                          className="hover:bg-[var(--background-tertiary)] transition-colors"
                        >
                          <td className="px-4 py-4">
                            <span
                              className={cn(
                                'inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium',
                                colorClass
                              )}
                            >
                              <Icon className="h-3 w-3" />
                              {run.status}
                            </span>
                          </td>
                          <td className="px-4 py-4">
                            <p className="max-w-md truncate text-sm text-[var(--foreground)]">
                              {run.input_text}
                            </p>
                          </td>
                          <td className="px-4 py-4">
                            <span className="text-sm text-[var(--foreground-muted)]">
                              {new Date(run.created_at).toLocaleString()}
                            </span>
                          </td>
                          <td className="px-4 py-4">
                            <span className="text-sm text-[var(--foreground)]">
                              ${(run.total_cost_cents / 100).toFixed(2)}
                            </span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

