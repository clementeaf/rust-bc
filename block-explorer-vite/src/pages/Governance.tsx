import { useEffect, useState } from 'react';
import PageIntro from '../components/PageIntro';
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

const STATUS_COLORS: Record<string, string> = {
  Voting: 'bg-blue-100 text-blue-800',
  Passed: 'bg-green-100 text-green-800',
  Rejected: 'bg-red-100 text-red-800',
  Executed: 'bg-purple-100 text-purple-800',
  Cancelled: 'bg-gray-100 text-gray-600',
  Expired: 'bg-amber-100 text-amber-800',
};

const STATUS_LABELS: Record<string, string> = {
  Voting: 'En votación',
  Passed: 'Aprobada',
  Rejected: 'Rechazada',
  Executed: 'Ejecutada',
  Cancelled: 'Cancelada',
  Expired: 'Expirada',
};

export default function Governance() {
  const [params, setParams] = useState<ProtocolParam[]>([]);
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [tallies, setTallies] = useState<Record<number, TallyResult>>({});

  // Form state
  const [proposer, setProposer] = useState('did:cerulean:');
  const [actionType, setActionType] = useState<'param_change' | 'text'>('text');
  const [paramKey, setParamKey] = useState('min_tx_fee');
  const [paramValue, setParamValue] = useState('5');
  const [textTitle, setTextTitle] = useState('');
  const [textDesc, setTextDesc] = useState('');
  const [deposit, setDeposit] = useState('10000');
  const [description, setDescription] = useState('');
  const [submitMsg, setSubmitMsg] = useState('');
  const [submitErr, setSubmitErr] = useState('');

  // Vote state
  const [voteForms, setVoteForms] = useState<Record<number, { voter: string; power: string }>>({});
  const [voteMsg, setVoteMsg] = useState('');

  useEffect(() => {
    loadParams();
    loadProposals();
  }, []);

  async function loadParams() {
    try {
      setParams(await getGovernanceParams());
    } catch { /* empty */ }
  }

  async function loadProposals() {
    try {
      const list = await getProposals();
      setProposals(list);
      for (const p of list) {
        try {
          const t = await tallyVotes(p.id);
          setTallies((prev) => ({ ...prev, [p.id]: t }));
        } catch { /* empty */ }
      }
    } catch { /* empty */ }
  }

  async function handleSubmit() {
    setSubmitMsg('');
    setSubmitErr('');
    try {
      const action =
        actionType === 'param_change'
          ? { type: 'param_change', changes: [{ key: paramKey, value: Number(paramValue) }] }
          : { type: 'text', title: textTitle, description: textDesc };
      await submitProposal({
        proposer,
        description,
        deposit: Number(deposit),
        action,
      });
      setSubmitMsg('Propuesta enviada correctamente');
      setDescription('');
      setTextTitle('');
      setTextDesc('');
      loadProposals();
    } catch (e: any) {
      setSubmitErr(e?.message || 'Error al enviar propuesta');
    }
  }

  async function handleVote(proposalId: number, option: 'Yes' | 'No' | 'Abstain') {
    setVoteMsg('');
    const form = voteForms[proposalId] || { voter: '', power: '' };
    if (!form.voter || !form.power) return;
    try {
      await castVote(proposalId, {
        voter: form.voter,
        option,
        power: Number(form.power),
      });
      setVoteMsg(`Voto registrado en propuesta #${proposalId}`);
      const t = await tallyVotes(proposalId);
      setTallies((prev) => ({ ...prev, [proposalId]: t }));
    } catch (e: any) {
      setVoteMsg(e?.message || 'Error al votar');
    }
  }

  function updateVoteForm(id: number, field: 'voter' | 'power', val: string) {
    setVoteForms((prev) => ({
      ...prev,
      [id]: { ...prev[id], [field]: val },
    }));
  }

  function actionLabel(action: any): string {
    if (action?.ParamChange) {
      return action.ParamChange.changes
        .map((c: any) => `${c[0]} → ${typeof c[1] === 'object' ? JSON.stringify(c[1]) : c[1]}`)
        .join(', ');
    }
    if (action?.TextProposal) return action.TextProposal.title;
    return JSON.stringify(action);
  }

  return (
    <div className="space-y-8">
      <PageIntro
        title="Gobernanza On-Chain"
        description="Propuestas, votación ponderada por stake y parámetros del protocolo. Cualquier participante puede proponer cambios; los validadores votan con poder proporcional a su stake."
      />

      {/* ── Parámetros del Protocolo ──────────────────────────────── */}
      <section className="bg-white rounded-lg border shadow-sm p-6">
        <h2 className="text-lg font-semibold mb-1">Parámetros del Protocolo</h2>
        <p className="text-sm text-gray-500 mb-4">
          Valores actuales. Modificables exclusivamente mediante propuestas de gobernanza aprobadas.
        </p>
        {params.length === 0 ? (
          <p className="text-gray-400 text-sm">Cargando parámetros...</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-gray-500">
                  <th className="py-2 pr-4">Parámetro</th>
                  <th className="py-2">Valor</th>
                </tr>
              </thead>
              <tbody>
                {params.map((p) => (
                  <tr key={p.key} className="border-b last:border-0">
                    <td className="py-2 pr-4 font-mono text-xs">{p.key}</td>
                    <td className="py-2 font-semibold">{p.value}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {/* ── Enviar Propuesta ──────────────────────────────────────── */}
      <section className="bg-white rounded-lg border shadow-sm p-6">
        <h2 className="text-lg font-semibold mb-4">Enviar Propuesta</h2>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Proponente (DID)</label>
            <input
              className="w-full rounded border px-3 py-2 text-sm"
              value={proposer}
              onChange={(e) => setProposer(e.target.value)}
              placeholder="did:cerulean:abc123"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Depósito (NOTA)</label>
            <input
              type="number"
              className="w-full rounded border px-3 py-2 text-sm"
              value={deposit}
              onChange={(e) => setDeposit(e.target.value)}
            />
          </div>
        </div>

        <div className="mb-4">
          <label className="block text-sm font-medium text-gray-700 mb-1">Tipo de propuesta</label>
          <select
            className="rounded border px-3 py-2 text-sm"
            value={actionType}
            onChange={(e) => setActionType(e.target.value as any)}
          >
            <option value="text">Propuesta de texto (señalización)</option>
            <option value="param_change">Cambio de parámetro</option>
          </select>
        </div>

        {actionType === 'param_change' ? (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Parámetro</label>
              <select
                className="w-full rounded border px-3 py-2 text-sm"
                value={paramKey}
                onChange={(e) => setParamKey(e.target.value)}
              >
                {params.map((p) => (
                  <option key={p.key} value={p.key}>
                    {p.key} (actual: {p.value})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Nuevo valor</label>
              <input
                type="number"
                className="w-full rounded border px-3 py-2 text-sm"
                value={paramValue}
                onChange={(e) => setParamValue(e.target.value)}
              />
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Título</label>
              <input
                className="w-full rounded border px-3 py-2 text-sm"
                value={textTitle}
                onChange={(e) => setTextTitle(e.target.value)}
                placeholder="Ej: Propuesta de actualización de consenso"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Detalle de la propuesta</label>
              <textarea
                className="w-full rounded border px-3 py-2 text-sm"
                rows={2}
                value={textDesc}
                onChange={(e) => setTextDesc(e.target.value)}
                placeholder="Descripción detallada..."
              />
            </div>
          </div>
        )}

        <div className="mb-4">
          <label className="block text-sm font-medium text-gray-700 mb-1">Justificación</label>
          <textarea
            className="w-full rounded border px-3 py-2 text-sm"
            rows={2}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Por qué se propone este cambio..."
          />
        </div>

        <button
          onClick={handleSubmit}
          className="bg-blue-600 text-white px-4 py-2 rounded text-sm font-medium hover:bg-blue-700"
        >
          Enviar Propuesta
        </button>

        {submitMsg && <p className="mt-3 text-sm text-green-700 bg-green-50 rounded p-2">{submitMsg}</p>}
        {submitErr && <p className="mt-3 text-sm text-red-700 bg-red-50 rounded p-2">{submitErr}</p>}
      </section>

      {/* ── Propuestas Activas ────────────────────────────────────── */}
      <section className="bg-white rounded-lg border shadow-sm p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">Propuestas</h2>
          <button
            onClick={loadProposals}
            className="text-sm text-blue-600 hover:underline"
          >
            Actualizar
          </button>
        </div>

        {voteMsg && <p className="mb-4 text-sm text-blue-700 bg-blue-50 rounded p-2">{voteMsg}</p>}

        {proposals.length === 0 ? (
          <p className="text-gray-400 text-sm">No hay propuestas aún. Envía la primera arriba.</p>
        ) : (
          <div className="space-y-4">
            {proposals.map((p) => {
              const tally = tallies[p.id];
              const vf = voteForms[p.id] || { voter: '', power: '' };
              return (
                <div key={p.id} className="border rounded-lg p-4">
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <span className="font-semibold text-sm">Propuesta #{p.id}</span>
                      <span className={`ml-2 text-xs px-2 py-0.5 rounded-full font-medium ${STATUS_COLORS[p.status] || 'bg-gray-100'}`}>
                        {STATUS_LABELS[p.status] || p.status}
                      </span>
                    </div>
                    <span className="text-xs text-gray-400">Depósito: {p.deposit.toLocaleString()} NOTA</span>
                  </div>

                  <p className="text-sm text-gray-700 mb-1">{p.description || '(sin descripción)'}</p>

                  <div className="text-xs text-gray-500 mb-2">
                    <span>Proponente: <span className="font-mono">{p.proposer}</span></span>
                    <span className="mx-2">·</span>
                    <span>Acción: {actionLabel(p.action)}</span>
                  </div>

                  {/* Tally bar */}
                  {tally && tally.total_voted_power > 0 && (
                    <div className="mb-3">
                      <div className="flex gap-1 h-4 rounded overflow-hidden mb-1">
                        {tally.yes_power > 0 && (
                          <div
                            className="bg-green-500 rounded-l"
                            style={{ width: `${(tally.yes_power / tally.total_voted_power) * 100}%` }}
                          />
                        )}
                        {tally.no_power > 0 && (
                          <div
                            className="bg-red-500"
                            style={{ width: `${(tally.no_power / tally.total_voted_power) * 100}%` }}
                          />
                        )}
                        {tally.abstain_power > 0 && (
                          <div
                            className="bg-gray-300 rounded-r"
                            style={{ width: `${(tally.abstain_power / tally.total_voted_power) * 100}%` }}
                          />
                        )}
                      </div>
                      <div className="flex text-xs gap-4 text-gray-600">
                        <span className="text-green-700">Sí: {tally.yes_power.toLocaleString()}</span>
                        <span className="text-red-700">No: {tally.no_power.toLocaleString()}</span>
                        <span className="text-gray-500">Abstención: {tally.abstain_power.toLocaleString()}</span>
                        <span>·</span>
                        <span>Quórum: {tally.quorum_reached ? '✓ Alcanzado' : '✗ No alcanzado'}</span>
                        <span>·</span>
                        <span className={tally.passed ? 'text-green-700 font-semibold' : 'text-red-600'}>
                          {tally.passed ? '✓ Aprobada' : '✗ No aprobada'}
                        </span>
                      </div>
                    </div>
                  )}

                  {/* Vote form — only for Voting proposals */}
                  {p.status === 'Voting' && (
                    <div className="bg-gray-50 rounded p-3 mt-2">
                      <p className="text-xs font-medium text-gray-600 mb-2">Votar en esta propuesta</p>
                      <div className="flex flex-wrap gap-2 items-end">
                        <input
                          className="rounded border px-2 py-1 text-xs w-48"
                          placeholder="DID del votante"
                          value={vf.voter}
                          onChange={(e) => updateVoteForm(p.id, 'voter', e.target.value)}
                        />
                        <input
                          type="number"
                          className="rounded border px-2 py-1 text-xs w-28"
                          placeholder="Stake (power)"
                          value={vf.power}
                          onChange={(e) => updateVoteForm(p.id, 'power', e.target.value)}
                        />
                        <button
                          onClick={() => handleVote(p.id, 'Yes')}
                          className="bg-green-600 text-white px-3 py-1 rounded text-xs font-medium hover:bg-green-700"
                        >
                          Sí
                        </button>
                        <button
                          onClick={() => handleVote(p.id, 'No')}
                          className="bg-red-600 text-white px-3 py-1 rounded text-xs font-medium hover:bg-red-700"
                        >
                          No
                        </button>
                        <button
                          onClick={() => handleVote(p.id, 'Abstain')}
                          className="bg-gray-500 text-white px-3 py-1 rounded text-xs font-medium hover:bg-gray-600"
                        >
                          Abstención
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
