export type InvoiceStatus = 'Pending' | 'Funded' | 'Paid' | 'Defaulted';

export interface Invoice {
  id: number;
  owner: string;
  debtor: string;
  amount: bigint;
  dueDate: number;
  description: string;
  status: InvoiceStatus;
  createdAt: number;
  fundedAt: number;
  paidAt: number;
  poolContract: string;
}

export interface InvestorPosition {
  deposited: bigint;
  available: bigint;
  deployed: bigint;
  earned: bigint;
  depositCount: number;
}

export interface PoolConfig {
  invoiceContract: string;
  admin: string;
  yieldBps: number;
}

export interface PoolTokenTotals {
  totalDeposited: bigint;
  totalDeployed: bigint;
  totalPaidOut: bigint;
}

export interface FundedInvoice {
  invoiceId: number;
  sme: string;
  /** Stablecoin contract used for this invoice */
  token: string;
  principal: bigint;
  committed: bigint;
  fundedAt: number;
  dueDate: number;
  repaid: boolean;
}

export type WalletState = {
  address: string | null;
  connected: boolean;
  network: string;
};
