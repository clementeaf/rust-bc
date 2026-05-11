import { useEffect, useState, useCallback } from 'react';
import PageIntro from '../components/PageIntro';
import { getAuditEvents, type AuditEntry } from '../lib/api';

const ACTION_OPTIONS = [
  '',
  'http_request',
  'block_mined',
  'wallet_created',
  'token_transfer',
  'token_staked',
  'token_unstaked',
  'chaincode_installed',
  'chaincode_upgraded',
  'did_registered',
  'did_revoked',
  'credential_stored',
  'credential_revoked',
  'channel_created',
  'proposal_submitted',
  'proposal_voted',
];

const ACTION_COLORS: Record<string, string> = {
  http_request: 'bg-neutral-100 text-neutral-600',
  block_mined: 'bg-blue-100 text-blue-800',
  wallet_created: 'bg-emerald-100 text-emerald-800',
  token_transfer: 'bg-amber-100 text-amber-800',
  token_staked: 'bg-purple-100 text-purple-800',
  token_unstaked: 'bg-orange-100 text-orange-800',
  chaincode_installed: 'bg-cyan-100 text-cyan-800',
  chaincode_upgraded: 'bg-teal-100 text-teal-800',
  did_registered: 'bg-indigo-100 text-indigo-800',
  did_revoked: 'bg-red-100 text-red-800',
  credential_stored: 'bg-lime-100 text-lime-800',
  credential_revoked: 'bg-rose-100 text-rose-800',
  channel_created: 'bg-sky-100 text-sky-800',
  proposal_submitted: 'bg-violet-100 text-violet-800',
  proposal_voted: 'bg-fuchsia-100 text-fuchsia-800',
};

export default function Compliance() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Filters
  const [actionFilter, setActionFilter] = useState('');
  const [orgFilter, setOrgFilter] = useState('');

  const fetchData = useCallback(async () => {
    try {
      const params: Record<string, string | number> = { limit: 500 };
      if (actionFilter) params.action = actionFilter;
      if (orgFilter) params.org_id = orgFilter;
      const data = await getAuditEvents(params);
      setEntries(data);
      setError('');
    } catch (e: any) {
      setError(e.message || 'Error loading audit events');
    } finally {
      setLoading(false);
    }
  }, [actionFilter, orgFilter]);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 10_000);
    return () => clearInterval(interval);
  }, [fetchData]);

  // Summary indicators
  const domainEvents = entries.filter((e) => e.action !== 'http_request');
  const blocksMined = entries.filter((e) => e.action === 'block_mined').length;
  const didMutations = entries.filter(
    (e) => e.action === 'did_registered' || e.action === 'did_revoked',
  ).length;
  const ccDeployments = entries.filter(
    (e) => e.action === 'chaincode_installed' || e.action === 'chaincode_upgraded',
  ).length;
  const failedRequests = entries.filter(
    (e) => e.action === 'http_request' && e.status_code >= 400,
  ).length;

  return (
    <>
      <PageIntro
        title="Compliance"
        description="Audit trail en tiempo real. Eventos de dominio y requests HTTP registrados para cumplimiento ISO 27001."
      />

      {/* Indicators */}
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-3 mb-6">
        <Indicator label="Eventos totales" value={entries.length} />
        <Indicator label="Bloques minados" value={blocksMined} />
        <Indicator label="Mutaciones DID" value={didMutations} />
        <Indicator label="Chaincode deploys" value={ccDeployments} />
        <Indicator label="Requests fallidos" value={failedRequests} color={failedRequests > 0 ? 'text-red-600' : undefined} />
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 mb-4">
        <select
          value={actionFilter}
          onChange={(e) => setActionFilter(e.target.value)}
          className="border border-neutral-200 rounded-lg px-3 py-1.5 text-sm bg-white"
        >
          <option value="">Todas las acciones</option>
          {ACTION_OPTIONS.filter(Boolean).map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>
        <input
          type="text"
          placeholder="Filtrar por org_id..."
          value={orgFilter}
          onChange={(e) => setOrgFilter(e.target.value)}
          className="border border-neutral-200 rounded-lg px-3 py-1.5 text-sm w-48"
        />
        <span className="text-xs text-neutral-400 self-center">
          Auto-refresh cada 10s
        </span>
      </div>

      {error && (
        <p className="text-sm text-red-500 mb-4">{error}</p>
      )}

      {loading ? (
        <p className="text-sm text-neutral-400">Cargando...</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-neutral-400 border-b border-neutral-200">
                <th className="py-2 pr-3">Timestamp</th>
                <th className="py-2 pr-3">Accion</th>
                <th className="py-2 pr-3">Org</th>
                <th className="py-2 pr-3">Method</th>
                <th className="py-2 pr-3">Path</th>
                <th className="py-2 pr-3">Status</th>
                <th className="py-2 pr-3">Duracion</th>
                <th className="py-2">Metadata</th>
              </tr>
            </thead>
            <tbody>
              {(actionFilter ? entries : domainEvents.length > 0 ? domainEvents : entries)
                .slice(0, 200)
                .map((e, i) => (
                  <tr key={`${e.trace_id}-${i}`} className="border-b border-neutral-100 hover:bg-neutral-50">
                    <td className="py-1.5 pr-3 text-xs text-neutral-500 whitespace-nowrap">
                      {new Date(e.timestamp).toLocaleString()}
                    </td>
                    <td className="py-1.5 pr-3">
                      <span
                        className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                          ACTION_COLORS[e.action] || 'bg-neutral-100 text-neutral-600'
                        }`}
                      >
                        {e.action}
                      </span>
                    </td>
                    <td className="py-1.5 pr-3 text-xs">{e.org_id || '-'}</td>
                    <td className="py-1.5 pr-3 text-xs font-mono">{e.method || '-'}</td>
                    <td className="py-1.5 pr-3 text-xs font-mono truncate max-w-[200px]">{e.path || '-'}</td>
                    <td className="py-1.5 pr-3 text-xs">
                      {e.status_code > 0 ? (
                        <span className={e.status_code >= 400 ? 'text-red-600 font-semibold' : ''}>
                          {e.status_code}
                        </span>
                      ) : '-'}
                    </td>
                    <td className="py-1.5 pr-3 text-xs text-neutral-400">
                      {e.duration_ms > 0 ? `${e.duration_ms}ms` : '-'}
                    </td>
                    <td className="py-1.5 text-xs text-neutral-500 truncate max-w-[250px]">
                      {e.metadata || '-'}
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
          {entries.length === 0 && (
            <p className="text-sm text-neutral-400 py-6 text-center">
              Sin eventos de audit registrados.
            </p>
          )}
        </div>
      )}
    </>
  );
}

function Indicator({ label, value, color }: { label: string; value: number; color?: string }) {
  return (
    <div className="bg-white border border-neutral-200 rounded-xl px-4 py-3">
      <p className="text-xs text-neutral-400">{label}</p>
      <p className={`text-2xl font-bold ${color || 'text-neutral-800'}`}>{value}</p>
    </div>
  );
}
