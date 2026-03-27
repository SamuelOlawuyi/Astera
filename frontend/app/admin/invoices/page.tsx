'use client';

import { useEffect, useState, useCallback } from 'react';
import { useStore } from '@/lib/store';
import { getInvoice, getInvoiceCount, buildInitCoFundingTx, submitTx } from '@/lib/contracts';
import { formatUSDC, truncateAddress, formatDate } from '@/lib/stellar';
import type { Invoice } from '@/lib/types';

export default function AdminInvoicesPage() {
  const { wallet } = useStore();
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const loadInvoices = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const count = await getInvoiceCount();
      const all: Invoice[] = [];
      for (let i = 1; i <= count; i++) {
        const inv = await getInvoice(i);
        // Only show pending invoices for this section
        if (inv.status === 'Pending') {
          all.push(inv);
        }
      }
      setInvoices(all);
    } catch (e) {
      setError('Failed to load pending invoices.');
      console.error(e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadInvoices();
  }, [loadInvoices]);

  async function handleApprove(invoice: Invoice) {
    if (!wallet.address) return;

    // Simple confirmation
    if (
      !confirm(
        `Are you sure you want to approve and fund Invoice #${invoice.id} for ${formatUSDC(invoice.amount)}?`,
      )
    ) {
      return;
    }

    setActionLoading(invoice.id);
    setError(null);
    setSuccess(null);

    try {
      const xdr = await buildInitCoFundingTx({
        admin: wallet.address,
        invoiceId: invoice.id,
        principal: invoice.amount,
        sme: invoice.owner,
        dueDate: invoice.dueDate,
      });

      const freighter = await import('@stellar/freighter-api');
      const { signedTxXdr, error: signError } = await freighter.signTransaction(xdr, {
        networkPassphrase: 'Test SDF Network ; September 2015',
        address: wallet.address,
      });

      if (signError) throw new Error(signError.message || 'Signing rejected.');

      await submitTx(signedTxXdr);
      setSuccess(`Invoice #${invoice.id} has been approved for co-funding.`);
      await loadInvoices();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Failed to approve invoice.';
      setError(msg);
      console.error(e);
    } finally {
      setActionLoading(null);
    }
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2">Pending Invoices</h1>
        <p className="text-brand-muted text-sm">
          Review and approve new invoice applications for co-funding.
        </p>
      </div>

      {error && (
        <div className="p-4 bg-red-900/20 border border-red-800/50 rounded-xl text-red-500 text-sm flex items-center justify-between">
          <span>{error}</span>
          <button onClick={() => setError(null)} className="text-lg">
            &times;
          </button>
        </div>
      )}

      {success && (
        <div className="p-4 bg-green-900/20 border border-green-800/50 rounded-xl text-green-500 text-sm flex items-center justify-between">
          <span>{success}</span>
          <button onClick={() => setSuccess(null)} className="text-lg">
            &times;
          </button>
        </div>
      )}

      <div className="bg-brand-card border border-brand-border rounded-2xl overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-brand-border bg-brand-dark/50">
                <th className="px-6 py-4 font-semibold text-brand-muted uppercase tracking-wider">
                  ID
                </th>
                <th className="px-6 py-4 font-semibold text-brand-muted uppercase tracking-wider">
                  Applicant
                </th>
                <th className="px-6 py-4 font-semibold text-brand-muted uppercase tracking-wider">
                  Amount
                </th>
                <th className="px-6 py-4 font-semibold text-brand-muted uppercase tracking-wider">
                  Due Date
                </th>
                <th className="px-6 py-4 font-semibold text-brand-muted uppercase tracking-wider">
                  Action
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-brand-border">
              {loading ? (
                [1, 2, 3].map((i) => (
                  <tr key={i} className="animate-pulse">
                    <td colSpan={5} className="px-6 py-8">
                      <div className="h-4 bg-brand-dark rounded w-full" />
                    </td>
                  </tr>
                ))
              ) : invoices.length === 0 ? (
                <tr>
                  <td colSpan={5} className="px-6 py-12 text-center text-brand-muted italic">
                    No pending invoice applications found.
                  </td>
                </tr>
              ) : (
                invoices.map((inv) => (
                  <tr key={inv.id} className="hover:bg-brand-dark/30 transition-colors">
                    <td className="px-6 py-4 font-mono">#{inv.id}</td>
                    <td className="px-6 py-4">
                      <div className="flex flex-col">
                        <span className="font-medium text-white">{inv.debtor}</span>
                        <span className="text-xs text-brand-muted">
                          {truncateAddress(inv.owner)}
                        </span>
                      </div>
                    </td>
                    <td className="px-6 py-4 font-bold text-white whitespace-nowrap">
                      {formatUSDC(inv.amount)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="flex flex-col">
                        <span>{formatDate(inv.dueDate)}</span>
                        <span className="text-xs text-brand-muted">
                          {Math.ceil((inv.dueDate * 1000 - Date.now()) / 86400000)} days remaining
                        </span>
                      </div>
                    </td>
                    <td className="px-6 py-4">
                      <button
                        onClick={() => handleApprove(inv)}
                        disabled={actionLoading !== null}
                        className="px-4 py-2 bg-brand-gold text-brand-dark text-xs font-bold rounded-lg hover:bg-brand-amber transition-colors disabled:opacity-50 whitespace-nowrap"
                      >
                        {actionLoading === inv.id ? 'Processing...' : 'Approve & Fund'}
                      </button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
