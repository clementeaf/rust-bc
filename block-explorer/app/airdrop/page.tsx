'use client';

import { useEffect, useState } from 'react';
import {
  getAirdropStatistics,
  getAirdropTiers,
  getClaimHistory,
  getEligibleNodes,
  getEligibilityInfo,
  claimAirdrop,
  type AirdropStatistics,
  type AirdropTier,
  type ClaimRecord,
  type EligibilityInfo,
  type NodeTracking,
} from '@/lib/api';

/**
 * Página principal del Dashboard de Airdrop
 */
export default function AirdropPage() {
  const [stats, setStats] = useState<AirdropStatistics | null>(null);
  const [tiers, setTiers] = useState<AirdropTier[]>([]);
  const [eligibleNodes, setEligibleNodes] = useState<NodeTracking[]>([]);
  const [claimHistory, setClaimHistory] = useState<ClaimRecord[]>([]);
  const [searchAddress, setSearchAddress] = useState('');
  const [eligibilityInfo, setEligibilityInfo] = useState<EligibilityInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [claiming, setClaiming] = useState(false);
  const [claimMessage, setClaimMessage] = useState<{ type: 'success' | 'error'; message: string } | null>(null);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 30000); // Refresh every 30 seconds
    return () => clearInterval(interval);
  }, []);

  async function loadData() {
    try {
      setLoading(true);
      const [statsData, tiersData, eligibleData, historyData] = await Promise.all([
        getAirdropStatistics(),
        getAirdropTiers(),
        getEligibleNodes(),
        getClaimHistory(20),
      ]);
      setStats(statsData);
      setTiers(tiersData);
      setEligibleNodes(eligibleData);
      setClaimHistory(historyData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load airdrop data');
    } finally {
      setLoading(false);
    }
  }

  async function handleSearch() {
    if (!searchAddress.trim()) {
      setEligibilityInfo(null);
      return;
    }

    try {
      const info = await getEligibilityInfo(searchAddress.trim());
      setEligibilityInfo(info);
      setError(null);
    } catch (err) {
      setEligibilityInfo(null);
      setError(err instanceof Error ? err.message : 'Node not found or not tracked');
    }
  }

  async function handleClaim(nodeAddress: string) {
    if (!confirm(`¿Estás seguro de que quieres reclamar el airdrop para ${nodeAddress.substring(0, 16)}...?`)) {
      return;
    }

    try {
      setClaiming(true);
      setClaimMessage(null);
      const result = await claimAirdrop(nodeAddress);
      setClaimMessage({
        type: 'success',
        message: `Airdrop reclamado exitosamente! ${result.airdrop_amount} tokens. Transaction ID: ${result.transaction_id.substring(0, 16)}...`,
      });
      await loadData(); // Reload data
    } catch (err) {
      setClaimMessage({
        type: 'error',
        message: err instanceof Error ? err.message : 'Failed to claim airdrop',
      });
    } finally {
      setClaiming(false);
    }
  }

  function formatAddress(address: string): string {
    return `${address.substring(0, 12)}...${address.substring(address.length - 8)}`;
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function formatUptime(seconds: number): string {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    if (days > 0) {
      return `${days}d ${hours}h`;
    }
    return `${hours}h`;
  }

  if (loading && !stats) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Cargando datos de airdrop...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <h1 className="text-3xl font-bold text-gray-900 mb-8">Airdrop Dashboard</h1>

        {claimMessage && (
          <div
            className={`mb-6 p-4 rounded-lg ${
              claimMessage.type === 'success'
                ? 'bg-green-50 border border-green-200 text-green-800'
                : 'bg-red-50 border border-red-200 text-red-800'
            }`}
          >
            {claimMessage.message}
          </div>
        )}

        {error && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg">
            {error}
          </div>
        )}

        {/* Estadísticas Generales */}
        {stats && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-sm font-medium text-gray-500">Total Nodos</h3>
              <p className="text-3xl font-bold text-gray-900 mt-2">{stats.total_nodes}</p>
            </div>
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-sm font-medium text-gray-500">Nodos Elegibles</h3>
              <p className="text-3xl font-bold text-green-600 mt-2">{stats.eligible_nodes}</p>
            </div>
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-sm font-medium text-gray-500">Claims Realizados</h3>
              <p className="text-3xl font-bold text-blue-600 mt-2">{stats.claimed_nodes}</p>
            </div>
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-sm font-medium text-gray-500">Total Distribuido</h3>
              <p className="text-3xl font-bold text-purple-600 mt-2">
                {stats.total_distributed.toLocaleString()} tokens
              </p>
            </div>
          </div>
        )}

        {/* Búsqueda de Elegibilidad */}
        <div className="bg-white rounded-lg shadow p-6 mb-8">
          <h2 className="text-xl font-bold text-gray-900 mb-4">Verificar Elegibilidad</h2>
          <div className="flex gap-4">
            <input
              type="text"
              value={searchAddress}
              onChange={(e) => setSearchAddress(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
              placeholder="Ingresa la dirección del nodo..."
              className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
            <button
              onClick={handleSearch}
              className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              Buscar
            </button>
          </div>

          {eligibilityInfo && (
            <div className="mt-6 p-4 bg-gray-50 rounded-lg">
              <h3 className="text-lg font-semibold mb-4">
                Estado de Elegibilidad: {eligibilityInfo.is_eligible ? (
                  <span className="text-green-600">✅ Elegible</span>
                ) : (
                  <span className="text-red-600">❌ No Elegible</span>
                )}
              </h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <p className="text-sm text-gray-600">Tier:</p>
                  <p className="font-semibold">Tier {eligibilityInfo.tier}</p>
                </div>
                <div>
                  <p className="text-sm text-gray-600">Cantidad Estimada:</p>
                  <p className="font-semibold">{eligibilityInfo.estimated_amount.toLocaleString()} tokens</p>
                </div>
                <div>
                  <p className="text-sm text-gray-600">Bloques Validados:</p>
                  <p className="font-semibold">{eligibilityInfo.blocks_validated}</p>
                </div>
                <div>
                  <p className="text-sm text-gray-600">Uptime:</p>
                  <p className="font-semibold">{eligibilityInfo.uptime_days} días</p>
                </div>
              </div>
              <div className="mt-4 pt-4 border-t border-gray-200">
                <h4 className="font-semibold mb-2">Requisitos:</h4>
                <ul className="space-y-1 text-sm">
                  <li>
                    {eligibilityInfo.requirements.meets_blocks_requirement ? '✅' : '❌'}{' '}
                    Mínimo {eligibilityInfo.requirements.min_blocks_validated} bloques
                    ({eligibilityInfo.requirements.current_blocks} actuales)
                  </li>
                  <li>
                    {eligibilityInfo.requirements.meets_uptime_requirement ? '✅' : '❌'}{' '}
                    Mínimo {eligibilityInfo.requirements.min_uptime_days} días de uptime
                    ({eligibilityInfo.requirements.current_uptime_days} actuales)
                  </li>
                  <li>
                    {eligibilityInfo.requirements.meets_position_requirement ? '✅' : '❌'}{' '}
                    Entre los primeros {eligibilityInfo.requirements.max_eligible_nodes} nodos
                  </li>
                </ul>
              </div>
              {eligibilityInfo.is_eligible && (
                <button
                  onClick={() => handleClaim(eligibilityInfo.node_address)}
                  disabled={claiming}
                  className="mt-4 px-6 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 disabled:opacity-50"
                >
                  {claiming ? 'Reclamando...' : 'Reclamar Airdrop'}
                </button>
              )}
            </div>
          )}
        </div>

        {/* Tiers */}
        {tiers.length > 0 && (
          <div className="bg-white rounded-lg shadow p-6 mb-8">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Tiers de Airdrop</h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {tiers.map((tier) => (
                <div key={tier.tier_id} className="p-4 border border-gray-200 rounded-lg">
                  <h3 className="font-semibold text-lg mb-2">
                    Tier {tier.tier_id}: {tier.name}
                  </h3>
                  <p className="text-sm text-gray-600 mb-2">
                    Bloques {tier.min_block_index} - {tier.max_block_index}
                  </p>
                  <p className="text-lg font-bold text-blue-600">
                    Base: {tier.base_amount.toLocaleString()} tokens
                  </p>
                  <p className="text-sm text-gray-600 mt-2">
                    +{tier.bonus_per_block} tokens/bloque (máx 100)
                  </p>
                  <p className="text-sm text-gray-600">
                    +{tier.bonus_per_uptime_day} tokens/día (máx 30 días)
                  </p>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Nodos Elegibles */}
        {eligibleNodes.length > 0 && (
          <div className="bg-white rounded-lg shadow p-6 mb-8">
            <h2 className="text-xl font-bold text-gray-900 mb-4">
              Nodos Elegibles ({eligibleNodes.length})
            </h2>
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Dirección
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Tier
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Bloques
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Uptime
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Acción
                    </th>
                  </tr>
                </thead>
                <tbody className="bg-white divide-y divide-gray-200">
                  {eligibleNodes.slice(0, 20).map((node) => (
                    <tr key={node.node_address}>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-mono">
                        {formatAddress(node.node_address)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        Tier {node.eligibility_tier}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        {node.blocks_validated}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        {formatUptime(node.uptime_seconds)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        <button
                          onClick={() => handleClaim(node.node_address)}
                          disabled={claiming}
                          className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
                        >
                          Reclamar
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Historial de Claims */}
        {claimHistory.length > 0 && (
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Historial de Claims</h2>
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Nodo
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Cantidad
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Tier
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Fecha
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      Estado
                    </th>
                  </tr>
                </thead>
                <tbody className="bg-white divide-y divide-gray-200">
                  {claimHistory.map((claim, idx) => (
                    <tr key={idx}>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-mono">
                        {formatAddress(claim.node_address)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm font-semibold">
                        {claim.airdrop_amount.toLocaleString()} tokens
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        Tier {claim.tier_id}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        {formatTimestamp(claim.claim_timestamp)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm">
                        {claim.verified ? (
                          <span className="px-2 py-1 bg-green-100 text-green-800 rounded">
                            ✅ Verificado
                          </span>
                        ) : (
                          <span className="px-2 py-1 bg-yellow-100 text-yellow-800 rounded">
                            ⏳ Pendiente
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

