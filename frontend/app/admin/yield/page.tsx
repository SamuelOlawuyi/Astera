'use client';

import { useEffect, useState } from 'react';
import { useStore } from '@/lib/store';
import { getPoolConfig, buildSetYieldTx, submitTx } from '@/lib/contracts';

export default function AdminYieldPage() {
  const { wallet, poolConfig, setPoolConfig } = useStore();
  const [newYield, setNewYield] = useState('');
  const [loading, setLoading] = useState(false);
  const [txLoading, setTxLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      setLoading(true);
      try {
        const config = await getPoolConfig();
        setPoolConfig(config);
        setNewYield((config.yieldBps / 100).toString());
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [setPoolConfig]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!wallet.address) return;

    const bps = Math.round(parseFloat(newYield) * 100);

    // Validation
    if (isNaN(bps) || bps < 0 || bps > 5000) {
      setError('Yield must be between 0% and 50% (5000 bps).');
      return;
    }

    setTxLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const xdr = await buildSetYieldTx(wallet.address, bps);

      const freighter = await import('@stellar/freighter-api');
      const { signedTxXdr, error: signError } = await freighter.signTransaction(xdr, {
        networkPassphrase: 'Test SDF Network ; September 2015',
        address: wallet.address,
      });

      if (signError) throw new Error(signError.message || 'Signing rejected.');

      await submitTx(signedTxXdr);
      setSuccess(`Yield rate updated to ${newYield}% (${bps} bps).`);

      const updatedConfig = await getPoolConfig();
      setPoolConfig(updatedConfig);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Failed to update yield rate.';
      setError(msg);
      console.error(e);
    } finally {
      setTxLoading(false);
    }
  }

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2">Yield Rate Management</h1>
        <p className="text-brand-muted text-sm">
          Configure the annual yield rate for the Astera liquidity pool.
        </p>
      </div>

      <div className="p-8 bg-brand-card border border-brand-border rounded-2xl shadow-sm">
        <label className="block text-sm font-semibold text-brand-muted mb-6 uppercase tracking-wider">
          Current Configuration
        </label>

        <div className="flex items-center gap-8 mb-8">
          <div className="flex flex-col">
            <span className="text-4xl font-bold text-white">
              {loading
                ? '...'
                : poolConfig?.yieldBps
                  ? (poolConfig.yieldBps / 100).toFixed(2)
                  : '0.00'}
              %
            </span>
            <span className="text-xs text-brand-muted">Annual Percentage Yield</span>
          </div>
          <div className="h-12 w-px bg-brand-border" />
          <div className="flex flex-col">
            <span className="text-4xl font-bold text-brand-gold">
              {loading ? '...' : poolConfig?.yieldBps}
            </span>
            <span className="text-xs text-brand-muted">Basis Points (bps)</span>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="space-y-6 pt-6 border-t border-brand-border">
          <div>
            <label className="block text-sm font-medium text-white mb-2">New Yield Rate (%)</label>
            <div className="relative">
              <input
                type="number"
                step="0.01"
                min="0"
                max="50"
                value={newYield}
                onChange={(e) => setNewYield(e.target.value)}
                className="w-full bg-brand-dark border border-brand-border rounded-xl px-4 py-3 text-white placeholder-brand-muted focus:outline-none focus:border-brand-gold text-lg"
                placeholder="e.g. 8.5"
                required
              />
              <span className="absolute right-4 top-1/2 -translate-y-1/2 text-brand-muted font-bold">
                %
              </span>
            </div>
            <p className="mt-2 text-xs text-brand-muted">
              Example: 8.5% is equivalent to 850 basis points.
            </p>
          </div>

          {error && (
            <div className="p-3 bg-red-900/20 border border-red-800/50 rounded-xl text-red-500 text-sm">
              {error}
            </div>
          )}
          {success && (
            <div className="p-3 bg-green-900/20 border border-green-800/50 rounded-xl text-green-500 text-sm">
              {success}
            </div>
          )}

          <button
            type="submit"
            disabled={txLoading || loading}
            className="w-full py-4 bg-brand-gold text-brand-dark font-bold rounded-xl hover:bg-brand-amber transition-all shadow-lg active:scale-[0.98] disabled:opacity-50"
          >
            {txLoading ? 'Updating Rate...' : 'Update Yield Rate'}
          </button>
        </form>
      </div>

      <div className="p-6 bg-brand-dark border border-brand-border rounded-2xl text-xs text-brand-muted space-y-2">
        <p className="font-bold text-white mb-1 uppercase tracking-tighter">Safety Controls:</p>
        <p>• The contract enforces a maximum yield of 50.00% (5000 bps).</p>
        <p>• Changes take effect immediately for all active and new invoices.</p>
        <p>• Yield is calculated based on linear accrual between funding and repayment.</p>
      </div>
    </div>
  );
}
