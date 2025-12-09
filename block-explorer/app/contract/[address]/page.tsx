'use client';

import { useEffect, useState } from 'react';
import { useParams } from 'next/navigation';
import { getContract, type SmartContract } from '@/lib/api';
import Link from 'next/link';

/**
 * Página de detalle de contrato inteligente
 */
export default function ContractPage() {
  const params = useParams();
  const address = params.address as string;
  const [contract, setContract] = useState<SmartContract | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (address) {
      loadContract();
    }
  }, [address]);

  async function loadContract() {
    try {
      setLoading(true);
      const data = await getContract(address);
      setContract(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Contract not found');
    } finally {
      setLoading(false);
    }
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading contract...</p>
        </div>
      </div>
    );
  }

  if (error || !contract) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-600 text-xl mb-4">Error: {error || 'Contract not found'}</p>
          <Link href="/contracts" className="text-primary-600 hover:text-primary-800">
            ← Back to Contracts
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <Link href="/contracts" className="text-primary-600 hover:text-primary-800 mb-4 inline-block">
          ← Back to Contracts
        </Link>

        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h1 className="text-3xl font-bold text-gray-900 mb-4">Contract Details</h1>
          <dl className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-gray-500">Address</dt>
              <dd className="mt-1 text-sm text-gray-900 font-mono break-all">{contract.address}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Created At</dt>
              <dd className="mt-1 text-sm text-gray-900">{formatTimestamp(contract.created_at)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Last Updated</dt>
              <dd className="mt-1 text-sm text-gray-900">{formatTimestamp(contract.updated_at)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Update Sequence</dt>
              <dd className="mt-1 text-sm text-gray-900">{contract.update_sequence}</dd>
            </div>
          </dl>
        </div>

        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">Contract State</h2>
          <pre className="bg-gray-50 p-4 rounded-lg overflow-x-auto text-sm">
            {JSON.stringify(contract.state, null, 2)}
          </pre>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-xl font-semibold text-gray-900 mb-4">Contract Code</h2>
          <pre className="bg-gray-50 p-4 rounded-lg overflow-x-auto text-sm font-mono">
            {contract.code}
          </pre>
        </div>
      </main>
    </div>
  );
}

