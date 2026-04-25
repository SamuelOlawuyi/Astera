import { safeSerialize, safeStringify } from '@/lib/stellar';

describe('safeSerialize / safeStringify', () => {
  it('passes through plain JSON-safe values unchanged', () => {
    expect(safeSerialize(null)).toBeNull();
    expect(safeSerialize(undefined)).toBeUndefined();
    expect(safeSerialize('hello')).toBe('hello');
    expect(safeSerialize(42)).toBe(42);
    expect(safeSerialize(true)).toBe(true);
  });

  it('converts a bare bigint to its string representation', () => {
    expect(safeSerialize(10n)).toBe('10');
    expect(safeSerialize(2n ** 100n)).toBe('1267650600228229401496703205376');
  });

  it('recurses into arrays and objects', () => {
    const input = {
      amount: 50_000_000n,
      nested: { earned: 0n, label: 'USDC' },
      arr: [1n, 2n, 'three'],
    };
    expect(safeSerialize(input)).toEqual({
      amount: '50000000',
      nested: { earned: '0', label: 'USDC' },
      arr: ['1', '2', 'three'],
    });
  });

  it('makes contract-like response objects safe for JSON.stringify', () => {
    const position = {
      deposited: 10_000_000_000n,
      available: 5_000_000_000n,
      deployed: 5_000_000_000n,
      earned: 123_456n,
      depositCount: 3,
    };
    // Sanity: native JSON.stringify throws on bigint.
    expect(() => JSON.stringify(position)).toThrow(TypeError);
    // safeStringify should succeed and produce a valid JSON document.
    const str = safeStringify(position);
    expect(() => JSON.parse(str)).not.toThrow();
    expect(JSON.parse(str)).toMatchObject({
      deposited: '10000000000',
      earned: '123456',
      depositCount: 3,
    });
  });

  it('does not touch non-plain objects like Date', () => {
    const d = new Date('2026-01-01T00:00:00Z');
    // Expect the same instance back (passthrough), so JSON.stringify applies
    // its own toJSON semantics.
    expect(safeSerialize(d)).toBe(d);
  });
});
