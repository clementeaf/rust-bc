import { useState, useEffect, useCallback, type ReactElement } from 'react'
import {
  listCredentials,
  listIdentities,
  type Credential,
  type IdentityRecord,
} from '../lib/api'
import { fmtDate, timeAgo } from '../lib/format'

// ── Helpers ─────────────────────────────────────────────────────────────────

function extractName(did: string): string {
  const slug = did.split(':').pop() || ''
  return slug.replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
}

function docStatus(c: Credential): 'vigente' | 'expirado' | 'revocado' {
  if (c.revoked_at) return 'revocado'
  if (c.expires_at > 0 && c.expires_at < Date.now() / 1000) return 'expirado'
  return 'vigente'
}

const STATUS_STYLES = {
  vigente: { bg: 'bg-emerald-100', text: 'text-emerald-700', label: 'Vigente' },
  expirado: { bg: 'bg-amber-100', text: 'text-amber-700', label: 'Expirado' },
  revocado: { bg: 'bg-red-100', text: 'text-red-700', label: 'Revocado' },
}

// ── Component ───────────────────────────────────────────────────────────────

export default function Credentials(): ReactElement {
  const [credentials, setCredentials] = useState<Credential[]>([])
  const [_identities, setIdentities] = useState<IdentityRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const [selected, setSelected] = useState<Credential | null>(null)
  const [search, setSearch] = useState('')
  const [filterStatus, setFilterStatus] = useState<'' | 'vigente' | 'expirado' | 'revocado'>('')

  const fetchAll = useCallback(async () => {
    try {
      const [creds, ids] = await Promise.all([listCredentials(), listIdentities()])
      setCredentials(creds)
      setIdentities(ids)
      setError('')
    } catch (e: any) {
      setError(e.message || 'Error cargando datos')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { fetchAll() }, [fetchAll])

  const filtered = credentials.filter((c) => {
    const status = docStatus(c)
    if (filterStatus && status !== filterStatus) return false
    if (!search) return true
    const q = search.toLowerCase()
    return (
      c.cred_type.toLowerCase().includes(q) ||
      extractName(c.issuer_did).toLowerCase().includes(q) ||
      extractName(c.subject_did).toLowerCase().includes(q) ||
      c.id.toLowerCase().includes(q)
    )
  })

  const counts = {
    total: credentials.length,
    vigente: credentials.filter((c) => docStatus(c) === 'vigente').length,
    expirado: credentials.filter((c) => docStatus(c) === 'expirado').length,
    revocado: credentials.filter((c) => docStatus(c) === 'revocado').length,
  }

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-neutral-900 tracking-tight">Documentos y Credenciales</h1>
          <p className="text-xs text-neutral-400 mt-0.5">
            Registro de todos los documentos firmados electronicamente en la plataforma
          </p>
        </div>
      </div>

      {/* Stats + filters */}
      <div className="flex items-center gap-3 mb-3 flex-wrap">
        <StatPill label="Total" value={counts.total} active={!filterStatus} onClick={() => setFilterStatus('')} />
        <StatPill label="Vigentes" value={counts.vigente} color="text-emerald-600" active={filterStatus === 'vigente'} onClick={() => setFilterStatus(filterStatus === 'vigente' ? '' : 'vigente')} />
        <StatPill label="Expirados" value={counts.expirado} color="text-amber-600" active={filterStatus === 'expirado'} onClick={() => setFilterStatus(filterStatus === 'expirado' ? '' : 'expirado')} />
        <StatPill label="Revocados" value={counts.revocado} color="text-red-500" active={filterStatus === 'revocado'} onClick={() => setFilterStatus(filterStatus === 'revocado' ? '' : 'revocado')} />
        <div className="flex-1" />
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Buscar por tipo, emisor o titular..."
          className="border border-neutral-200 rounded-lg px-3 py-1.5 text-sm w-56 focus:outline-none focus:ring-2 focus:ring-main-500"
        />
      </div>

      {error && <p className="text-sm text-red-500 mb-3">{error}</p>}

      {/* Table — full width */}
      <div className="bg-white border border-neutral-200 rounded-xl overflow-hidden">
        {loading ? (
          <p className="text-sm text-neutral-400 px-5 py-6 text-center">Cargando...</p>
        ) : filtered.length === 0 ? (
          <p className="text-sm text-neutral-400 px-5 py-6 text-center">
            {credentials.length === 0 ? 'Sin documentos registrados.' : 'Sin resultados para el filtro aplicado.'}
          </p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[10px] text-neutral-400 uppercase tracking-wider border-b border-neutral-200">
                <th className="px-4 py-2">Documento</th>
                <th className="px-4 py-2">Firmado por</th>
                <th className="px-4 py-2">Titular</th>
                <th className="px-4 py-2">Estado</th>
                <th className="px-4 py-2">Fecha</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((c) => {
                const status = docStatus(c)
                const st = STATUS_STYLES[status]
                return (
                  <tr
                    key={c.id}
                    onClick={() => setSelected(c)}
                    className="border-b border-neutral-100 cursor-pointer hover:bg-main-50/40"
                  >
                    <td className="px-4 py-2.5">
                      <p className="text-xs font-medium text-neutral-800">{c.cred_type}</p>
                    </td>
                    <td className="px-4 py-2.5 text-xs text-neutral-600">{extractName(c.issuer_did)}</td>
                    <td className="px-4 py-2.5 text-xs text-neutral-600">{extractName(c.subject_did)}</td>
                    <td className="px-4 py-2.5">
                      <span className={`text-[9px] px-1.5 py-0.5 rounded font-medium ${st.bg} ${st.text}`}>{st.label}</span>
                    </td>
                    <td className="px-4 py-2.5 text-xs text-neutral-500">{timeAgo(c.issued_at)}</td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        )}
      </div>

      {/* Detail drawer */}
      {selected && (
        <>
          <div className="fixed inset-0 z-50 bg-black/20 animate-backdrop" onClick={() => setSelected(null)} />
          <div className="fixed inset-y-0 right-0 z-50 w-full max-w-md bg-white shadow-xl flex flex-col animate-slide-in">
            {/* Header */}
            <div className="px-5 py-4 border-b border-neutral-200">
              <div className="flex items-center justify-between">
                <p className="text-base font-bold text-neutral-900">{selected.cred_type}</p>
                <button onClick={() => setSelected(null)} className="p-1.5 rounded-lg hover:bg-neutral-100">
                  <svg className="w-5 h-5 text-neutral-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <p className="text-xs text-neutral-400 mt-0.5">Documento con firma electronica avanzada</p>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto px-5 py-4">
              {/* Status banner */}
              {(() => {
                const status = docStatus(selected)
                const st = STATUS_STYLES[status]
                return (
                  <div className={`rounded-lg px-3 py-2.5 mb-4 ${st.bg}`}>
                    <p className={`text-xs font-medium ${st.text}`}>
                      {status === 'vigente' ? 'Documento vigente — firma electronica valida'
                        : status === 'expirado' ? 'Documento expirado'
                        : 'Documento revocado — firma invalidada'}
                    </p>
                  </div>
                )
              })()}

              {/* Fields */}
              <div className="space-y-3 mb-5">
                <Field label="Tipo de documento" value={selected.cred_type} />
                <Field label="Firmado por" value={extractName(selected.issuer_did)} sub={selected.issuer_did} />
                <Field label="Titular" value={extractName(selected.subject_did)} sub={selected.subject_did} />
                <Field label="Fecha de firma" value={fmtDate(selected.issued_at)} />
                {selected.expires_at > 0 && <Field label="Vencimiento" value={fmtDate(selected.expires_at)} />}
                {selected.revoked_at && <Field label="Revocado" value={fmtDate(selected.revoked_at)} />}
              </div>

              {/* Claims */}
              {selected.claims && Object.keys(selected.claims).length > 0 && (
                <div className="mb-5">
                  <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Contenido del documento</p>
                  <div className="bg-neutral-50 border border-neutral-200 rounded-lg p-3 space-y-2">
                    {Object.entries(selected.claims).map(([k, v]) => (
                      <div key={k} className="flex justify-between items-baseline">
                        <span className="text-xs text-neutral-500 capitalize">{k.replace(/_/g, ' ')}</span>
                        <span className="text-xs text-neutral-800 font-medium text-right">{String(v)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Proof */}
              <div>
                <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Garantia criptografica</p>
                <div className="bg-neutral-900 rounded-lg p-3 space-y-1.5">
                  <ProofRow label="Algoritmo" value="ML-DSA-65 (FIPS 204)" />
                  <ProofRow label="ID documento" value={selected.id} />
                  <ProofRow label="Registro" value="Blockchain Cerulean Ledger" />
                  <ProofRow label="Inmutabilidad" value="Hash SHA-256 sellado en bloque" />
                  <ProofRow label="Verificable" value="Cualquier nodo puede verificar" />
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  )
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
  )
}

function Field({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div>
      <p className="text-[10px] text-neutral-400 uppercase tracking-wider">{label}</p>
      <p className="text-sm text-neutral-800">{value}</p>
      {sub && <p className="text-[10px] text-neutral-400 font-mono truncate">{sub}</p>}
    </div>
  )
}

function ProofRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-[10px] text-neutral-500">{label}</span>
      <span className="text-[10px] text-emerald-400 font-mono truncate ml-2 max-w-[180px]">{value}</span>
    </div>
  )
}
