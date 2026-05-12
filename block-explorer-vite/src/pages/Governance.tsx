import { useEffect, useState, useCallback } from 'react';
import {
  getGovernanceParams,
  getProposals,
  submitProposal,
  castVote,
  tallyVotes,
  type ProtocolParam,
  type Proposal,
  type TallyResult,
} from '../lib/api';
import { fmtDate } from '../lib/format';

// ── Helpers ─────────────────────────────────────────────────────────────────

const STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  Voting:    { bg: 'bg-blue-100',    text: 'text-blue-700',    label: 'En votacion' },
  Passed:    { bg: 'bg-emerald-100', text: 'text-emerald-700', label: 'Aprobada' },
  Rejected:  { bg: 'bg-red-100',     text: 'text-red-700',     label: 'Rechazada' },
  Executed:  { bg: 'bg-violet-100',  text: 'text-violet-700',  label: 'Ejecutada' },
  Cancelled: { bg: 'bg-neutral-100', text: 'text-neutral-500', label: 'Cancelada' },
  Expired:   { bg: 'bg-amber-100',   text: 'text-amber-700',   label: 'Expirada' },
};

function actionSummary(action: any): string {
  if (action?.ParamChange) {
    return action.ParamChange.changes.map((c: any) => `${c[0]} → ${typeof c[1] === 'object' ? JSON.stringify(c[1]) : c[1]}`).join(', ');
  }
  if (action?.TextProposal) return action.TextProposal.title || 'Propuesta de texto';
  return 'Propuesta';
}

function pct(part: number, total: number): number {
  return total > 0 ? Math.round((part / total) * 100) : 0;
}

// ── Component ───────────────────────────────────────────────────────────────

export default function Governance() {
  const [params, setParams] = useState<ProtocolParam[]>([]);
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [tallies, setTallies] = useState<Record<number, TallyResult>>({});
  const [loading, setLoading] = useState(true);

  // Selected proposal
  const [selected, setSelected] = useState<Proposal | null>(null);

  // Create form
  const [showCreate, setShowCreate] = useState(false);
  const [newDesc, setNewDesc] = useState('');
  const [newTitle, setNewTitle] = useState('');
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState('');

  // Vote
  const [voterName, setVoterName] = useState('');
  const [voteMsg, setVoteMsg] = useState('');

  const fetchAll = useCallback(async () => {
    try {
      const [p, list] = await Promise.all([getGovernanceParams(), getProposals()]);
      setParams(p);
      setProposals(list);
      const t: Record<number, TallyResult> = {};
      for (const prop of list) {
        try { t[prop.id] = await tallyVotes(prop.id); } catch { /* skip */ }
      }
      setTallies(t);
    } catch { /* empty */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchAll(); }, [fetchAll]);

  const handleCreate = async () => {
    if (!newTitle.trim()) return;
    setCreating(true);
    setCreateError('');
    try {
      await submitProposal({
        proposer: 'ciudadano',
        description: newDesc || newTitle,
        deposit: 10000,
        action: { type: 'text', title: newTitle, description: newDesc },
      });
      setNewTitle('');
      setNewDesc('');
      setShowCreate(false);
      await fetchAll();
    } catch (e: any) {
      setCreateError(e.message || 'Error');
    } finally {
      setCreating(false);
    }
  };

  const handleVote = async (proposalId: number, option: 'Yes' | 'No' | 'Abstain') => {
    if (!voterName.trim()) { setVoteMsg('Ingresa tu nombre para votar'); return; }
    setVoteMsg('');
    try {
      await castVote(proposalId, { voter: voterName.trim(), option, power: 1 });
      const t = await tallyVotes(proposalId);
      setTallies((prev) => ({ ...prev, [proposalId]: t }));
      setVoteMsg('Voto registrado');
    } catch (e: any) {
      setVoteMsg(e.message || 'Error al votar');
    }
  };

  const voting = proposals.filter((p) => p.status === 'Voting');
  const closed = proposals.filter((p) => p.status !== 'Voting');

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-neutral-900 tracking-tight">Gobernanza Digital</h1>
          <p className="text-xs text-neutral-400 mt-0.5">
            Propuestas, votacion y decisiones colectivas con transparencia criptografica
          </p>
        </div>
        <button
          onClick={() => setShowCreate(!showCreate)}
          className="bg-main-500 text-white px-4 py-1.5 rounded-lg text-sm font-medium hover:bg-main-600"
        >
          {showCreate ? 'Cancelar' : 'Nueva propuesta'}
        </button>
      </div>

      {/* Create form */}
      {showCreate && (
        <div className="bg-main-50 border border-main-200 rounded-xl px-5 py-4 mb-4">
          <p className="text-xs text-neutral-500 mb-2">Presenta una propuesta para decision colectiva.</p>
          <div className="space-y-2 mb-3">
            <input
              value={newTitle}
              onChange={(e) => setNewTitle(e.target.value)}
              placeholder="Titulo de la propuesta"
              autoFocus
              className="w-full border border-neutral-200 rounded-lg px-3 py-2 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-main-500"
            />
            <textarea
              value={newDesc}
              onChange={(e) => setNewDesc(e.target.value)}
              placeholder="Descripcion y justificacion..."
              rows={2}
              className="w-full border border-neutral-200 rounded-lg px-3 py-2 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-main-500 resize-none"
            />
          </div>
          <button
            onClick={() => void handleCreate()}
            disabled={creating || !newTitle.trim()}
            className="bg-main-600 text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-main-700 disabled:opacity-50"
          >
            {creating ? 'Enviando...' : 'Enviar propuesta'}
          </button>
          {createError && <p className="text-xs text-red-500 mt-2">{createError}</p>}
        </div>
      )}

      {/* Proposals table — full width */}
      <div className="bg-white border border-neutral-200 rounded-xl overflow-hidden">
        {loading ? (
          <p className="text-sm text-neutral-400 px-5 py-6 text-center">Cargando...</p>
        ) : proposals.length === 0 ? (
          <p className="text-sm text-neutral-400 px-5 py-6 text-center">Sin propuestas. Crea la primera.</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[10px] text-neutral-400 uppercase tracking-wider border-b border-neutral-200">
                <th className="px-4 py-2">#</th>
                <th className="px-4 py-2">Propuesta</th>
                <th className="px-4 py-2">Proponente</th>
                <th className="px-4 py-2">Estado</th>
                <th className="px-4 py-2">Resultado</th>
                <th className="px-4 py-2">Fecha</th>
              </tr>
            </thead>
            <tbody>
              {proposals.map((p) => {
                const st = STATUS_STYLES[p.status] || STATUS_STYLES.Cancelled;
                const t = tallies[p.id];
                const total = t?.total_voted_power || 1;
                return (
                  <tr
                    key={p.id}
                    onClick={() => setSelected(p)}
                    className="border-b border-neutral-100 cursor-pointer hover:bg-main-50/40"
                  >
                    <td className="px-4 py-2.5 text-xs text-neutral-400">{p.id}</td>
                    <td className="px-4 py-2.5">
                      <p className="text-xs font-medium text-neutral-800">{actionSummary(p.action)}</p>
                      <p className="text-[10px] text-neutral-400 truncate max-w-[300px]">{p.description}</p>
                    </td>
                    <td className="px-4 py-2.5 text-xs text-neutral-600">{p.proposer}</td>
                    <td className="px-4 py-2.5">
                      <span className={`text-[9px] px-1.5 py-0.5 rounded font-medium ${st.bg} ${st.text}`}>{st.label}</span>
                    </td>
                    <td className="px-4 py-2.5 w-32">
                      {t && t.total_voted_power > 0 ? (
                        <div>
                          <div className="flex h-1.5 rounded-full overflow-hidden bg-neutral-100">
                            {t.yes_power > 0 && <div className="bg-emerald-500" style={{ width: `${pct(t.yes_power, total)}%` }} />}
                            {t.no_power > 0 && <div className="bg-red-500" style={{ width: `${pct(t.no_power, total)}%` }} />}
                          </div>
                          <p className="text-[9px] text-neutral-400 mt-0.5">{pct(t.yes_power, total)}% a favor</p>
                        </div>
                      ) : (
                        <span className="text-[9px] text-neutral-300">Sin votos</span>
                      )}
                    </td>
                    <td className="px-4 py-2.5 text-xs text-neutral-500">{fmtDate(p.submitted_at)}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      {/* Detail drawer */}
      {selected && (
        <>
          <div className="fixed inset-0 z-50 bg-black/20 animate-backdrop" onClick={() => setSelected(null)} />
          <div className="fixed inset-y-0 right-0 z-50 w-full max-w-lg bg-white shadow-xl flex flex-col animate-slide-in">
            {/* Header */}
            <div className="px-5 py-4 border-b border-neutral-200">
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <p className="text-base font-bold text-neutral-900">{actionSummary(selected.action)}</p>
                  <p className="text-xs text-neutral-400 mt-0.5">Propuesta #{selected.id}</p>
                </div>
                <div className="flex items-center gap-2">
                  <StatusBadge status={selected.status} />
                  <button onClick={() => setSelected(null)} className="p-1.5 rounded-lg hover:bg-neutral-100">
                    <svg className="w-5 h-5 text-neutral-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto px-5 py-4">
              <p className="text-sm text-neutral-600 mb-4">{selected.description}</p>

              {/* Metadata */}
              <div className="grid grid-cols-3 gap-3 mb-4">
                <MetaItem label="Proponente" value={selected.proposer} />
                <MetaItem label="Deposito" value={`${selected.deposit.toLocaleString()} NOTA`} />
                <MetaItem label="Fecha" value={fmtDate(selected.submitted_at)} />
              </div>

              {/* Tally */}
              {(() => {
                const t = tallies[selected.id];
                if (!t) return <p className="text-xs text-neutral-400 mb-4">Cargando resultados...</p>;
                const total = t.total_voted_power || 1;
                return (
                  <div className="mb-5">
                    <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Resultados</p>
                    <div className="flex h-3 rounded-full overflow-hidden bg-neutral-100 mb-2">
                      {t.yes_power > 0 && <div className="bg-emerald-500" style={{ width: `${pct(t.yes_power, total)}%` }} />}
                      {t.no_power > 0 && <div className="bg-red-500" style={{ width: `${pct(t.no_power, total)}%` }} />}
                      {t.abstain_power > 0 && <div className="bg-neutral-300" style={{ width: `${pct(t.abstain_power, total)}%` }} />}
                    </div>
                    <div className="flex gap-4 text-xs mb-3">
                      <span className="text-emerald-700">A favor: {t.yes_power} ({pct(t.yes_power, total)}%)</span>
                      <span className="text-red-600">En contra: {t.no_power} ({pct(t.no_power, total)}%)</span>
                      <span className="text-neutral-500">Abstencion: {t.abstain_power}</span>
                    </div>
                    <div className="grid grid-cols-2 gap-2">
                      <div className={`rounded-lg px-3 py-2 text-xs ${t.quorum_reached ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-600'}`}>
                        Quorum: {t.quorum_reached ? 'Alcanzado' : 'No alcanzado'}
                      </div>
                      <div className={`rounded-lg px-3 py-2 text-xs ${t.passed ? 'bg-emerald-50 text-emerald-700' : 'bg-neutral-50 text-neutral-500'}`}>
                        Resultado: {t.passed ? 'Aprobada' : 'No aprobada'}
                      </div>
                    </div>
                  </div>
                );
              })()}

              {/* Vote — only for Voting */}
              {selected.status === 'Voting' && (
                <div className="border-t border-neutral-100 pt-4 mb-5">
                  <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Emitir voto</p>
                  <input
                    value={voterName}
                    onChange={(e) => setVoterName(e.target.value)}
                    placeholder="Tu nombre"
                    className="w-full border border-neutral-200 rounded-lg px-3 py-2 text-sm mb-2 focus:outline-none focus:ring-2 focus:ring-main-500"
                  />
                  <div className="flex gap-2">
                    <button onClick={() => void handleVote(selected.id, 'Yes')} disabled={!voterName.trim()}
                      className="flex-1 bg-emerald-500 text-white py-2 rounded-lg text-sm font-medium hover:bg-emerald-600 disabled:opacity-50">A favor</button>
                    <button onClick={() => void handleVote(selected.id, 'No')} disabled={!voterName.trim()}
                      className="flex-1 bg-red-500 text-white py-2 rounded-lg text-sm font-medium hover:bg-red-600 disabled:opacity-50">En contra</button>
                    <button onClick={() => void handleVote(selected.id, 'Abstain')} disabled={!voterName.trim()}
                      className="flex-1 bg-neutral-400 text-white py-2 rounded-lg text-sm font-medium hover:bg-neutral-500 disabled:opacity-50">Abstencion</button>
                  </div>
                  {voteMsg && <p className="text-xs text-main-600 mt-2">{voteMsg}</p>}
                </div>
              )}

              {/* Params affected */}
              {selected.action?.ParamChange && (
                <div className="border-t border-neutral-100 pt-4 mb-5">
                  <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Parametros afectados</p>
                  <div className="bg-neutral-50 border border-neutral-200 rounded-lg p-3">
                    {selected.action.ParamChange.changes.map((c: any, i: number) => (
                      <div key={i} className="flex justify-between text-xs py-1">
                        <span className="text-neutral-500 font-mono">{c[0]}</span>
                        <span className="text-neutral-800 font-medium">{typeof c[1] === 'object' ? JSON.stringify(c[1]) : String(c[1])}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Crypto proof */}
              <div className="border-t border-neutral-100 pt-4">
                <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Garantia criptografica</p>
                <div className="bg-neutral-900 rounded-lg p-3 space-y-1.5">
                  <ProofRow label="Votos" value="Inmutables — registrados en blockchain" />
                  <ProofRow label="Conteo" value="Verificable por cualquier nodo" />
                  <ProofRow label="Algoritmo" value="ML-DSA-65 (FIPS 204)" />
                  <ProofRow label="Quorum" value="Ponderado por stake — no manipulable" />
                </div>
              </div>
            </div>
          </div>
        </>
      )}

      {/* Protocol params — compact bottom */}
      {params.length > 0 && (
        <div className="mt-4 bg-white border border-neutral-200 rounded-xl px-5 py-3">
          <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">
            Parametros del protocolo ({params.length})
          </p>
          <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-6 gap-2">
            {params.map((p) => (
              <div key={p.key} className="bg-neutral-50 rounded-lg px-3 py-1.5">
                <p className="text-[9px] text-neutral-400 font-mono truncate">{p.key}</p>
                <p className="text-xs font-semibold text-neutral-800">{p.value}</p>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Sub-components ──────────────────────────────────────────────────────────

function ProposalSection({ title, count, loading, children }: { title: string; count: number; loading: boolean; children: React.ReactNode }) {
  return (
    <div className="bg-white border border-neutral-200 rounded-xl px-4 py-3">
      <div className="flex items-center justify-between mb-2">
        <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold">{title}</p>
        <span className="text-[10px] text-neutral-400">{count}</span>
      </div>
      {loading ? (
        <p className="text-xs text-neutral-400 py-2">Cargando...</p>
      ) : count === 0 ? (
        <p className="text-xs text-neutral-400 py-2">Sin propuestas.</p>
      ) : (
        <div className="space-y-0.5">{children}</div>
      )}
    </div>
  );
}

function ProposalRow({ proposal, tally, active, onClick }: { proposal: Proposal; tally?: TallyResult; active: boolean; onClick: () => void }) {
  const st = STATUS_STYLES[proposal.status] || STATUS_STYLES.Cancelled;
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-3 py-2.5 rounded-lg transition-colors ${
        active ? 'bg-main-500 text-white' : 'hover:bg-neutral-50'
      }`}
    >
      <div className="flex items-center justify-between">
        <p className={`text-xs font-medium truncate flex-1 ${active ? 'text-white' : 'text-neutral-800'}`}>
          {actionSummary(proposal.action)}
        </p>
        <span className={`text-[9px] px-1.5 py-0.5 rounded font-medium ml-2 flex-shrink-0 ${
          active ? 'bg-white/20 text-white' : `${st.bg} ${st.text}`
        }`}>
          {st.label}
        </span>
      </div>
      {tally && tally.total_voted_power > 0 && (
        <div className="flex h-1 rounded-full overflow-hidden bg-neutral-200 mt-1.5">
          {tally.yes_power > 0 && <div className={`${active ? 'bg-white/60' : 'bg-emerald-500'}`} style={{ width: `${pct(tally.yes_power, tally.total_voted_power)}%` }} />}
          {tally.no_power > 0 && <div className={`${active ? 'bg-white/30' : 'bg-red-500'}`} style={{ width: `${pct(tally.no_power, tally.total_voted_power)}%` }} />}
        </div>
      )}
    </button>
  );
}

function StatusBadge({ status }: { status: string }) {
  const st = STATUS_STYLES[status] || STATUS_STYLES.Cancelled;
  return <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium flex-shrink-0 ${st.bg} ${st.text}`}>{st.label}</span>;
}

function MetaItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[9px] text-neutral-400 uppercase">{label}</p>
      <p className="text-xs text-neutral-700 font-medium">{value}</p>
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
