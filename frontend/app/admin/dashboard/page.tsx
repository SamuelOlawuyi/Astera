'use client';

import { useEffect, useState, useMemo } from 'react';
import { useStore } from '@/lib/store';
import { getPoolConfig, getInvoiceCount, getInvoice } from '@/lib/contracts';
import { formatUSDC } from '@/lib/stellar';
import type { Invoice } from '@/lib/types';

export default function AdminDashboardPage() {
  const { poolConfig, setPoolConfig } = useStore();
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadData() {
      setLoading(true);
      setError(null);
      try {
        const [config, count] = await Promise.all([getPoolConfig(), getInvoiceCount()]);
        setPoolConfig(config);

        const all: Invoice[] = [];
        for (let i = 1; i <= count; i++) {
          const inv = await getInvoice(i);
          all.push(inv);
        }
        setInvoices(all);
      } catch (e) {
        setError('Failed to load protocol statistics.');
        console.error(e);
      } finally {
        setLoading(false);
      }
    }

    loadData();
  }, [setPoolConfig]);

  const stats = useMemo(() => {
    if (!poolConfig) return null;

    const fundedInvoices = invoices.filter((i) => i.status !== 'Pending');
    const activeInvoices = invoices.filter((i) => i.status === 'Funded');
    const defaultedInvoices = invoices.filter((i) => i.status === 'Defaulted');

    const defaultRate =
      fundedInvoices.length > 0 ? (defaultedInvoices.length / fundedInvoices.length) * 100 : 0;

    return {
      tvl: poolConfig.totalDeposited,
      activeCount: activeInvoices.length,
      defaultRate: defaultRate.toFixed(2) + '%',
      totalDeployed: poolConfig.totalDeployed,
      totalPaidOut: poolConfig.totalPaidOut,
    };
  }, [poolConfig, invoices]);

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="h-10 w-48 bg-brand-card animate-pulse rounded-lg" />
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {[1, 2, 3].map((i) => (
            <div
              key={i}
              className="h-32 bg-brand-card animate-pulse rounded-2xl border border-brand-border"
            />
          ))}
        </div>
      </div>
    );
  }

  if (error || !stats) {
    return (
      <div className="p-6 bg-red-900/20 border border-red-800/50 rounded-2xl text-red-500">
        {error || 'Failed to calculate stats.'}
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2">Protocol Dashboard</h1>
        <p className="text-brand-muted text-sm">Real-time overview of the Astera liquidity pool.</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <StatCard
          label="Total TVL"
          value={formatUSDC(stats.tvl)}
          description="Total USDC deposited in the pool"
          trend="primary"
        />
        <StatCard
          label="Active Invoices"
          value={stats.activeCount.toString()}
          description="Invoices currently funded and not yet repaid"
        />
        <StatCard
          label="Default Rate"
          value={stats.defaultRate}
          description="Percentage of funded invoices that defaulted"
          trend={parseFloat(stats.defaultRate) > 5 ? 'danger' : 'success'}
        />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="p-6 bg-brand-card border border-brand-border rounded-2xl">
          <h2 className="text-lg font-semibold mb-4">Capital Deployment</h2>
          <div className="space-y-4">
            <div className="flex justify-between items-center text-sm">
              <span className="text-brand-muted">Currently Deployed</span>
              <span className="font-mono font-bold">{formatUSDC(stats.totalDeployed)}</span>
            </div>
            <div className="w-full bg-brand-dark h-2 rounded-full overflow-hidden">
              <div
                className="bg-brand-gold h-full transition-all duration-500"
                style={{
                  width: `${stats.tvl > 0n ? Number((stats.totalDeployed * 100n) / stats.tvl) : 0}%`,
                }}
              />
            </div>
            <p className="text-xs text-brand-muted">
              {stats.tvl > 0n ? Number((stats.totalDeployed * 100n) / stats.tvl) : 0}% of total
              liquidity is currently active in SMEs.
            </p>
          </div>
        </div>

        <div className="p-6 bg-brand-card border border-brand-border rounded-2xl">
          <h2 className="text-lg font-semibold mb-4">Cumulative Yield</h2>
          <div className="space-y-4">
            <div className="flex justify-between items-center text-sm">
              <span className="text-brand-muted">Total Paid Out</span>
              <span className="font-mono font-bold text-green-400">
                {formatUSDC(stats.totalPaidOut)}
              </span>
            </div>
            <p className="text-xs text-brand-muted">
              Total returns generated and distributed to investors to date.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  description,
  trend,
}: {
  label: string;
  value: string;
  description: string;
  trend?: 'primary' | 'danger' | 'success';
}) {
  return (
    <div className="p-6 bg-brand-card border border-brand-border rounded-2xl shadow-sm hover:border-brand-gold/30 transition-colors">
      <p className="text-xs font-bold text-brand-muted uppercase tracking-wider mb-2">{label}</p>
      <div className="flex items-baseline gap-2 mb-2">
        <p
          className={`text-4xl font-bold tracking-tight ${trend === 'primary' ? 'gradient-text' : trend === 'danger' ? 'text-red-500' : trend === 'success' ? 'text-green-500' : ''}`}
        >
          {value}
        </p>
      </div>
      <p className="text-xs text-brand-muted">{description}</p>
    </div>
  );
}
