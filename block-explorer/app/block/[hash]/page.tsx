'use client';

import { useEffect, useState } from 'react';
import { useParams } from 'next/navigation';
import { getBlockByHash, type Block } from '@/lib/api';
import Link from 'next/link';

export default function BlockPage() {
  const params = useParams();
  const hash = params.hash as string;
  const [block, setBlock] = useState<Block | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (hash) {
      loadBlock();
    }
  }, [hash]);

  async function loadBlock() {
    try {
      setLoading(true);
      const blockData = await getBlockByHash(hash);
      setBlock(blockData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Block not found');
    } finally {
      setLoading(false);
    }
  }

  function formatHash(hash: string): string {
    return hash;
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading block...</p>
        </div>
      </div>
    );
  }

  if (error || !block) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-600 text-xl mb-4">Error: {error || 'Block not found'}</p>
          <Link href="/" className="text-primary-600 hover:text-primary-800">
            ← Back to Home
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white shadow-sm">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <Link href="/" className="text-primary-600 hover:text-primary-800 mb-2 inline-block">
            ← Back to Home
          </Link>
          <h1 className="text-3xl font-bold text-gray-900">Block #{block.index}</h1>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">Block Information</h2>
          <dl className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-gray-500">Index</dt>
              <dd className="mt-1 text-sm text-gray-900">{block.index}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Timestamp</dt>
              <dd className="mt-1 text-sm text-gray-900">{formatTimestamp(block.timestamp)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Hash</dt>
              <dd className="mt-1 text-sm text-gray-900 font-mono break-all">{formatHash(block.hash)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Previous Hash</dt>
              <dd className="mt-1 text-sm text-gray-900 font-mono break-all">
                {block.previous_hash === '0' ? (
                  <span className="text-gray-400">Genesis Block</span>
                ) : (
                  <Link
                    href={`/block/${block.previous_hash}`}
                    className="text-primary-600 hover:text-primary-800"
                  >
                    {formatHash(block.previous_hash)}
                  </Link>
                )}
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Merkle Root</dt>
              <dd className="mt-1 text-sm text-gray-900 font-mono break-all">{formatHash(block.merkle_root)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Nonce</dt>
              <dd className="mt-1 text-sm text-gray-900">{block.nonce}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Transactions</dt>
              <dd className="mt-1 text-sm text-gray-900">{block.transactions.length}</dd>
            </div>
          </dl>
        </div>

        <div className="bg-white rounded-lg shadow">
          <div className="px-6 py-4 border-b border-gray-200">
            <h2 className="text-xl font-semibold text-gray-900">Transactions ({block.transactions.length})</h2>
          </div>
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    ID
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    From
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    To
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Amount
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Fee
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {block.transactions.map((tx) => (
                  <tr key={tx.id} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-mono text-gray-500">
                      {formatHash(tx.id)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-mono text-gray-500">
                      {formatHash(tx.from)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-mono text-gray-500">
                      {formatHash(tx.to)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                      {tx.amount}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {tx.fee}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </main>
    </div>
  );
}

