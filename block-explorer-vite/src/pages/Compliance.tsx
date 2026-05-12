import { useEffect, useState, useCallback } from 'react';
import { getAuditEvents, type AuditEntry } from '../lib/api';

// ── Action labels (human-readable Spanish) ──────────────────────────────────

const ACTION_LABELS: Record<string, string> = {
  http_request: 'Solicitud HTTP',
  block_mined: 'Bloque minado',
  wallet_created: 'Wallet creada',
  token_transfer: 'Transferencia',
  token_staked: 'Staking',
  token_unstaked: 'Unstaking',
  chaincode_installed: 'Chaincode instalado',
  chaincode_upgraded: 'Chaincode actualizado',
  did_registered: 'Identidad registrada',
  did_revoked: 'Identidad revocada',
  credential_stored: 'Documento firmado',
  credential_revoked: 'Documento revocado',
  channel_created: 'Canal creado',
  proposal_submitted: 'Propuesta enviada',
  proposal_voted: 'Voto emitido',
};

const ACTION_STYLES: Record<string, string> = {
  http_request: 'bg-neutral-100 text-neutral-600',
  block_mined: 'bg-blue-100 text-blue-700',
  wallet_created: 'bg-emerald-100 text-emerald-700',
  token_transfer: 'bg-amber-100 text-amber-700',
  token_staked: 'bg-purple-100 text-purple-700',
  token_unstaked: 'bg-orange-100 text-orange-700',
  chaincode_installed: 'bg-cyan-100 text-cyan-700',
  chaincode_upgraded: 'bg-teal-100 text-teal-700',
  did_registered: 'bg-indigo-100 text-indigo-700',
  did_revoked: 'bg-red-100 text-red-700',
  credential_stored: 'bg-lime-100 text-lime-700',
  credential_revoked: 'bg-rose-100 text-rose-700',
  channel_created: 'bg-sky-100 text-sky-700',
  proposal_submitted: 'bg-violet-100 text-violet-700',
  proposal_voted: 'bg-fuchsia-100 text-fuchsia-700',
};

const CATEGORY_FILTERS = [
  { value: '', label: 'Todos' },
  { value: 'identity', label: 'Identidad', actions: ['did_registered', 'did_revoked'] },
  { value: 'documents', label: 'Documentos', actions: ['credential_stored', 'credential_revoked'] },
  { value: 'governance', label: 'Gobernanza', actions: ['proposal_submitted', 'proposal_voted'] },
  { value: 'blockchain', label: 'Blockchain', actions: ['block_mined', 'wallet_created', 'token_transfer'] },
  { value: 'infra', label: 'Infraestructura', actions: ['chaincode_installed', 'chaincode_upgraded', 'channel_created'] },
  { value: 'errors', label: 'Errores', actions: [] }, // special: status >= 400
];

export default function Compliance() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [category, setCategory] = useState('');
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<AuditEntry | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const data = await getAuditEvents({ limit: 500 });
      setEntries(data);
      setError('');
    } catch (e: any) {
      setError(e.message || 'Error cargando eventos');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 10_000);
    return () => clearInterval(interval);
  }, [fetchData]);

  // Filter
  const filtered = entries.filter((e) => {
    // Category filter
    if (category) {
      const cat = CATEGORY_FILTERS.find((c) => c.value === category);
      if (cat) {
        if (category === 'errors') {
          if (e.status_code < 400) return false;
        } else if (cat.actions && !cat.actions.includes(e.action)) {
          return false;
        }
      }
    }
    // Hide http_request by default unless explicitly filtered
    if (!category && e.action === 'http_request') return false;
    // Search
    if (search) {
      const q = search.toLowerCase();
      return (
        e.action.toLowerCase().includes(q) ||
        (ACTION_LABELS[e.action] || '').toLowerCase().includes(q) ||
        e.path.toLowerCase().includes(q) ||
        e.org_id.toLowerCase().includes(q) ||
        (e.metadata || '').toLowerCase().includes(q)
      );
    }
    return true;
  });

  // Counts
  const domainCount = entries.filter((e) => e.action !== 'http_request').length;
  const errorCount = entries.filter((e) => e.status_code >= 400).length;
  const identityCount = entries.filter((e) => e.action === 'did_registered' || e.action === 'did_revoked').length;
  const docCount = entries.filter((e) => e.action === 'credential_stored' || e.action === 'credential_revoked').length;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-neutral-900 tracking-tight">Audit Trail</h1>
          <p className="text-xs text-neutral-400 mt-0.5">
            Registro inmutable de todas las operaciones — ISO 27001 · Auto-refresh 10s
          </p>
        </div>
      </div>

      {/* Stats */}
      <div className="flex items-center gap-2 mb-3 flex-wrap">
        <StatPill label="Eventos" value={domainCount} active={!category} onClick={() => setCategory('')} />
        <StatPill label="Identidad" value={identityCount} active={category === 'identity'} onClick={() => setCategory(category === 'identity' ? '' : 'identity')} color="text-indigo-600" />
        <StatPill label="Documentos" value={docCount} active={category === 'documents'} onClick={() => setCategory(category === 'documents' ? '' : 'documents')} color="text-lime-600" />
        <StatPill label="Gobernanza" value={entries.filter((e) => e.action === 'proposal_submitted' || e.action === 'proposal_voted').length} active={category === 'governance'} onClick={() => setCategory(category === 'governance' ? '' : 'governance')} color="text-violet-600" />
        <StatPill label="Blockchain" value={entries.filter((e) => e.action === 'block_mined').length} active={category === 'blockchain'} onClick={() => setCategory(category === 'blockchain' ? '' : 'blockchain')} color="text-blue-600" />
        <StatPill label="Errores" value={errorCount} active={category === 'errors'} onClick={() => setCategory(category === 'errors' ? '' : 'errors')} color={errorCount > 0 ? 'text-red-500' : undefined} />
        <div className="flex-1" />
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Buscar..."
          className="border border-neutral-200 rounded-lg px-3 py-1.5 text-sm w-48 focus:outline-none focus:ring-2 focus:ring-main-500"
        />
      </div>

      {error && <p className="text-sm text-red-500 mb-3">{error}</p>}

      {/* Table — flex-1 fills remaining space, internal scroll */}
      <div className="bg-white border border-neutral-200 rounded-xl overflow-hidden flex-1 min-h-0 flex flex-col">
        {loading ? (
          <p className="text-sm text-neutral-400 px-5 py-6 text-center">Cargando...</p>
        ) : filtered.length === 0 ? (
          <p className="text-sm text-neutral-400 px-5 py-6 text-center">Sin eventos registrados.</p>
        ) : (
          <div className="flex-1 min-h-0 overflow-y-auto">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-white z-10">
              <tr className="text-left text-[10px] text-neutral-400 uppercase tracking-wider border-b border-neutral-200">
                <th className="px-4 py-2">Evento</th>
                <th className="px-4 py-2">Organizacion</th>
                <th className="px-4 py-2">Recurso</th>
                <th className="px-4 py-2">Estado</th>
                <th className="px-4 py-2">Tiempo</th>
              </tr>
            </thead>
            <tbody>
              {filtered.slice(0, 200).map((e, i) => (
                <tr
                  key={`${e.trace_id}-${i}`}
                  onClick={() => setSelected(e)}
                  className="border-b border-neutral-100 cursor-pointer hover:bg-main-50/40"
                >
                  <td className="px-4 py-2.5">
                    <span className={`text-[9px] px-1.5 py-0.5 rounded-full font-medium ${ACTION_STYLES[e.action] || 'bg-neutral-100 text-neutral-600'}`}>
                      {ACTION_LABELS[e.action] || e.action}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 text-xs text-neutral-600">{e.org_id || '-'}</td>
                  <td className="px-4 py-2.5 text-xs text-neutral-500 font-mono truncate max-w-[250px]">{e.path || '-'}</td>
                  <td className="px-4 py-2.5">
                    {e.status_code > 0 ? (
                      <span className={`text-xs font-medium ${e.status_code >= 400 ? 'text-red-600' : e.status_code >= 200 && e.status_code < 300 ? 'text-emerald-600' : 'text-neutral-500'}`}>
                        {e.status_code}
                      </span>
                    ) : '-'}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-neutral-400 whitespace-nowrap">
                    {new Date(e.timestamp).toLocaleTimeString('es-CL')}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        )}
      </div>

      {/* Detail drawer */}
      {selected && (
        <>
          <div className="fixed inset-0 z-50 bg-black/20 animate-backdrop" onClick={() => setSelected(null)} />
          <div className="fixed inset-y-0 right-0 z-50 w-full max-w-md bg-white shadow-xl flex flex-col animate-slide-in">
            <div className="px-5 py-4 border-b border-neutral-200">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-base font-bold text-neutral-900">{ACTION_LABELS[selected.action] || selected.action}</p>
                  <p className="text-xs text-neutral-400 mt-0.5">Evento de audit trail</p>
                </div>
                <button onClick={() => setSelected(null)} className="p-1.5 rounded-lg hover:bg-neutral-100">
                  <svg className="w-5 h-5 text-neutral-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto px-5 py-4">
              {/* Status */}
              <div className={`rounded-lg px-3 py-2.5 mb-4 ${
                selected.status_code >= 400 ? 'bg-red-50' : selected.status_code >= 200 && selected.status_code < 300 ? 'bg-emerald-50' : 'bg-neutral-50'
              }`}>
                <p className={`text-xs font-medium ${
                  selected.status_code >= 400 ? 'text-red-700' : selected.status_code >= 200 && selected.status_code < 300 ? 'text-emerald-700' : 'text-neutral-500'
                }`}>
                  {selected.status_code >= 400 ? 'Operacion fallida' : selected.status_code >= 200 && selected.status_code < 300 ? 'Operacion exitosa' : 'Estado desconocido'}
                  {' — '}{selected.status_code}
                </p>
              </div>

              {/* Fields */}
              <div className="space-y-3 mb-5">
                <Field label="Accion" value={ACTION_LABELS[selected.action] || selected.action} />
                <Field label="Accion (codigo)" value={selected.action} />
                <Field label="Organizacion" value={selected.org_id || 'No especificada'} />
                <Field label="Metodo" value={selected.method || '-'} />
                <Field label="Recurso" value={selected.path || '-'} />
                <Field label="Codigo HTTP" value={String(selected.status_code || '-')} />
                <Field label="Duracion" value={selected.duration_ms > 0 ? `${selected.duration_ms}ms` : '-'} />
                <Field label="Timestamp" value={new Date(selected.timestamp).toLocaleString('es-CL')} />
                <Field label="IP origen" value={selected.source_ip || '-'} />
                <Field label="Trace ID" value={selected.trace_id} />
              </div>

              {/* Metadata */}
              {selected.metadata && (
                <div className="mb-5">
                  <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Metadata</p>
                  <div className="bg-neutral-50 border border-neutral-200 rounded-lg p-3">
                    <p className="text-xs text-neutral-700 font-mono break-all">{selected.metadata}</p>
                  </div>
                </div>
              )}

              {/* Compliance info */}
              <div>
                <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Cumplimiento normativo</p>
                <div className="bg-neutral-900 rounded-lg p-3 space-y-1.5">
                  <ProofRow label="Normativa" value="ISO 27001 — Gestion de seguridad" />
                  <ProofRow label="Registro" value="Inmutable en audit store" />
                  <ProofRow label="Retencion" value="Completa — sin eliminacion" />
                  <ProofRow label="Trazabilidad" value={selected.trace_id.slice(0, 16) + '...'} />
                  <ProofRow label="Exportable" value="CSV via /audit/export" />
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

// ── Sub-components ──────────────────────────────────────────────────────────

function StatPill({ label, value, color, active, onClick }: { label: string; value: number; color?: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`border rounded-lg px-3 py-1.5 transition-colors ${
        active ? 'border-main-300 bg-main-50' : 'border-neutral-200 bg-white hover:bg-neutral-50'
      }`}
    >
      <p className="text-[9px] text-neutral-400 uppercase">{label}</p>
      <p className={`text-base font-bold ${color || 'text-neutral-800'}`}>{value}</p>
    </button>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[10px] text-neutral-400 uppercase tracking-wider">{label}</p>
      <p className="text-sm text-neutral-800 font-mono break-all">{value}</p>
    </div>
  );
}

function ProofRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-[10px] text-neutral-500">{label}</span>
      <span className="text-[10px] text-emerald-400">{value}</span>
    </div>
  );
}
