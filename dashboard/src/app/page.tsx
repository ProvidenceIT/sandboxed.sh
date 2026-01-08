'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { toast } from '@/components/toast';
import { StatsCard } from '@/components/stats-card';
import { ConnectionStatus } from '@/components/connection-status';
import { RecentTasks } from '@/components/recent-tasks';
import { ShimmerStat } from '@/components/ui/shimmer';
import { getStats, StatsResponse } from '@/lib/api';
import { Activity, CheckCircle, DollarSign, Zap, Plus } from 'lucide-react';
import { formatCents } from '@/lib/utils';
import { SystemMonitor } from '@/components/system-monitor';

export default function OverviewPage() {
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [isActive, setIsActive] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    let hasShownError = false;

    const fetchStats = async () => {
      try {
        const data = await getStats();
        if (!mounted) return;
        setStats(data);
        setIsActive(data.active_tasks > 0);
        setError(null);
        setLoading(false);
        hasShownError = false;
      } catch (err) {
        if (!mounted) return;
        const message = err instanceof Error ? err.message : 'Failed to fetch stats';
        setError(message);
        setLoading(false);
        if (!hasShownError) {
          toast.error('Failed to connect to agent server');
          hasShownError = true;
        }
      }
    };

    fetchStats();
    const interval = setInterval(fetchStats, 3000);
    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return (
    <div className="flex min-h-screen">
      {/* Main content */}
      <div className="flex-1 flex flex-col p-6">
        {/* Header */}
        <div className="mb-6 flex items-start justify-between">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-xl font-semibold text-white">
                Global Monitor
              </h1>
              {isActive && (
                <span className="flex items-center gap-1.5 rounded-md bg-emerald-500/10 border border-emerald-500/20 px-2 py-1 text-[10px] font-medium text-emerald-400">
                  <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" />
                  LIVE
                </span>
              )}
            </div>
            <p className="mt-1 text-sm text-white/50">
              Real-time agent activity
            </p>
          </div>
          
          {/* Quick Actions */}
          <Link
            href="/control"
            className="flex items-center gap-2 rounded-lg bg-indigo-500/20 px-3 py-2 text-sm font-medium text-indigo-400 hover:bg-indigo-500/30 transition-colors"
          >
            <Plus className="h-4 w-4" />
            New Mission
          </Link>
        </div>

        {/* System Metrics Area */}
        <div className="flex-1 flex items-center justify-center rounded-2xl bg-white/[0.01] border border-white/[0.04] mb-6 min-h-[300px] p-6">
          <SystemMonitor className="w-full max-w-4xl" />
        </div>

        {/* Stats grid - at bottom */}
        <div className="grid grid-cols-4 gap-4">
          {loading ? (
            <>
              <ShimmerStat />
              <ShimmerStat />
              <ShimmerStat />
              <ShimmerStat />
            </>
          ) : (
            <>
              <StatsCard
                title="Total Tasks"
                value={stats?.total_tasks ?? 0}
                icon={Activity}
              />
              <StatsCard
                title="Active"
                value={stats?.active_tasks ?? 0}
                subtitle="running"
                icon={Zap}
                color={stats?.active_tasks ? 'accent' : 'default'}
              />
              <StatsCard
                title="Success Rate"
                value={`${((stats?.success_rate ?? 1) * 100).toFixed(0)}%`}
                icon={CheckCircle}
                color="success"
              />
              <StatsCard
                title="Total Cost"
                value={formatCents(stats?.total_cost_cents ?? 0)}
                icon={DollarSign}
              />
            </>
          )}
        </div>
      </div>

      {/* Right sidebar - no glass panel wrapper, just border */}
      <div className="w-80 h-screen border-l border-white/[0.06] p-4 flex flex-col overflow-hidden">
        <div className="flex-1 min-h-0 overflow-hidden">
          <RecentTasks />
        </div>
        <div className="mt-4 flex-shrink-0">
          <ConnectionStatus />
        </div>
      </div>
    </div>
  );
}
