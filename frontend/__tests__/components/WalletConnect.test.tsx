import React from 'react';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import WalletConnect from '@/components/WalletConnect';
import { useStore } from '@/lib/store';

// The component dynamically imports '@stellar/freighter-api' — mock both the
// real module and any dynamic-import requests to the same specifier.
const mockFreighter = {
  isConnected: jest.fn(),
  isAllowed: jest.fn(),
  setAllowed: jest.fn(),
  getAddress: jest.fn(),
};
jest.mock('@stellar/freighter-api', () => mockFreighter);

// Don't actually make contract calls anywhere downstream.
jest.mock('@/lib/contracts', () => ({}));

describe('WalletConnect', () => {
  beforeEach(() => {
    // Reset Zustand store between tests.
    act(() => {
      useStore.getState().disconnect();
    });
    jest.clearAllMocks();
  });

  it('shows the connect button when disconnected', () => {
    render(<WalletConnect />);
    expect(screen.getByRole('button', { name: /connect wallet/i })).toBeInTheDocument();
  });

  it('shows the truncated address and a disconnect button when connected', () => {
    act(() => {
      useStore.getState().setWallet({
        address: 'GABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDXYZ',
        connected: true,
        network: 'testnet',
      });
    });

    render(<WalletConnect />);
    // Truncated format is `${first6}...${last4}` — assert on both ends.
    expect(screen.getByText(/GABCDE.+DXYZ/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /disconnect/i })).toBeInTheDocument();
  });

  it('connects successfully through the freighter flow and updates the store', async () => {
    mockFreighter.isConnected.mockResolvedValue({ isConnected: true });
    mockFreighter.isAllowed.mockResolvedValue({ isAllowed: true });
    mockFreighter.getAddress.mockResolvedValue({ address: 'GTEST', error: undefined });

    render(<WalletConnect />);
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));

    // After the flow resolves, the store should reflect the connected state.
    expect(useStore.getState().wallet).toMatchObject({
      address: 'GTEST',
      connected: true,
    });
  });

  it('reports a clear error when Freighter is not installed', async () => {
    mockFreighter.isConnected.mockResolvedValue({ isConnected: false });

    render(<WalletConnect />);
    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent(/Freighter not detected/i);
    expect(useStore.getState().wallet.connected).toBe(false);
  });
});
