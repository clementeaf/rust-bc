import { useState, useEffect, useCallback, type ReactElement } from 'react'
import {
  createIdentity,
  listIdentities,
  getCredentialsBySubject,
  type IdentityRecord,
  type Credential,
} from '../lib/api'
import { timeAgo, fmtDate } from '../lib/format'

// ── Helpers ─────────────────────────────────────────────────────────────────

function extractName(did: string): string {
  const slug = did.split(':').pop() || ''
  return slug.replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
}

function initials(name: string): string {
  return name.split(' ').map((w) => w[0]).join('').slice(0, 2).toUpperCase()
}

function isOrg(did: string): boolean {
  const slug = did.split(':').pop() || ''
  return ['universidad', 'banco', 'registro', 'servicio', 'ministerio', 'hospital', 'empresa'].some((k) =>
    slug.includes(k),
  )
}

function docStatus(cred: Credential): 'vigente' | 'expirado' | 'revocado' {
  if (cred.revoked_at) return 'revocado'
  if (cred.expires_at > 0 && cred.expires_at < Date.now() / 1000) return 'expirado'
  return 'vigente'
}

const DOC_STATUS_STYLES = {
  vigente: 'bg-emerald-100 text-emerald-700',
  expirado: 'bg-amber-100 text-amber-700',
  revocado: 'bg-red-100 text-red-700',
}

// ── Component ───────────────────────────────────────────────────────────────

export default function Identity(): ReactElement {
  const [identities, setIdentities] = useState<IdentityRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  // Selected identity
  const [selected, setSelected] = useState<IdentityRecord | null>(null)
  const [docs, setDocs] = useState<Credential[]>([])
  const [docsLoading, setDocsLoading] = useState(false)

  // Detail drawer
  const [openDoc, setOpenDoc] = useState<Credential | null>(null)

  // Create
  const [showCreate, setShowCreate] = useState(false)
  const [newName, setNewName] = useState('')
  const [creating, setCreating] = useState(false)
  const [createError, setCreateError] = useState('')

  const fetchAll = useCallback(async () => {
    try {
      setIdentities(await listIdentities())
      setError('')
    } catch (e: any) {
      setError(e.message || 'Error')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { fetchAll() }, [fetchAll])

  const handleSelect = useCallback(async (id: IdentityRecord) => {
    setSelected(id)
    setDocs([])
    setDocsLoading(true)
    try {
      setDocs(await getCredentialsBySubject(id.did))
    } catch {
      setDocs([])
    } finally {
      setDocsLoading(false)
    }
  }, [])

  const handleCreate = async () => {
    if (!newName.trim()) return
    setCreating(true)
    setCreateError('')
    try {
      const slug = newName.trim().toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '')
      await createIdentity(`did:cerulean:${slug}`, 'active')
      setNewName('')
      setShowCreate(false)
      await fetchAll()
    } catch (e: any) {
      setCreateError(e.message || 'Error')
    } finally {
      setCreating(false)
    }
  }

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-neutral-900 tracking-tight">Identidad Digital</h1>
          <p className="text-xs text-neutral-400 mt-0.5">
            Firma documentos de instituciones publicas y privadas con validez legal y seguridad criptografica
          </p>
        </div>
        <button
          onClick={() => setShowCreate(!showCreate)}
          className="bg-main-500 text-white px-4 py-1.5 rounded-lg text-sm font-medium hover:bg-main-600"
        >
          {showCreate ? 'Cancelar' : 'Registrar identidad'}
        </button>
      </div>

      {/* Create */}
      {showCreate && (
        <div className="bg-main-50 border border-main-200 rounded-xl px-5 py-4 mb-4">
          <p className="text-xs text-neutral-500 mb-2">Registra tu identidad para firmar y recibir documentos digitales.</p>
          <div className="flex gap-3 items-end">
            <div className="flex-1">
              <label className="text-[10px] text-neutral-400 uppercase tracking-wider block mb-1">Nombre completo o razon social</label>
              <input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && void handleCreate()}
                placeholder="Ej: Maria Gonzalez o Banco Central de Chile"
                autoFocus
                className="w-full border border-neutral-200 rounded-lg px-3 py-2 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-main-500"
              />
            </div>
            <button
              onClick={() => void handleCreate()}
              disabled={creating || !newName.trim()}
              className="bg-main-600 text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-main-700 disabled:opacity-50 whitespace-nowrap"
            >
              {creating ? 'Registrando...' : 'Registrar'}
            </button>
          </div>
          {createError && <p className="text-xs text-red-500 mt-2">{createError}</p>}
        </div>
      )}

      {error && <p className="text-sm text-red-500 mb-3">{error}</p>}

      {/* Main: identities left, documents right */}
      <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr] gap-4">

        {/* Identity list */}
        <div className="bg-white border border-neutral-200 rounded-xl overflow-hidden">
          <div className="px-4 py-2.5 border-b border-neutral-100">
            <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold">
              Identidades registradas ({identities.length})
            </p>
          </div>
          <div className="overflow-y-auto max-h-[calc(100vh-16rem)]">
            {loading ? (
              <p className="text-xs text-neutral-400 px-4 py-4">Cargando...</p>
            ) : identities.length === 0 ? (
              <p className="text-xs text-neutral-400 px-4 py-4">Sin identidades. Registra la primera.</p>
            ) : (
              identities.map((id) => {
                const name = extractName(id.did)
                const active = selected?.did === id.did
                return (
                  <button
                    key={id.did}
                    onClick={() => handleSelect(id)}
                    className={`w-full flex items-center gap-3 px-4 py-3 text-left border-b border-neutral-50 transition-colors ${
                      active ? 'bg-main-500 text-white' : 'hover:bg-neutral-50'
                    }`}
                  >
                    <span className={`w-8 h-8 rounded-full flex items-center justify-center text-[10px] font-bold flex-shrink-0 ${
                      active ? 'bg-white/20 text-white' : isOrg(id.did) ? 'bg-main-100 text-main-600' : 'bg-emerald-100 text-emerald-600'
                    }`}>
                      {initials(name)}
                    </span>
                    <div className="flex-1 min-w-0">
                      <p className={`text-xs font-medium truncate ${active ? 'text-white' : 'text-neutral-800'}`}>{name}</p>
                      <p className={`text-[10px] truncate ${active ? 'text-white/60' : 'text-neutral-400'}`}>
                        {isOrg(id.did) ? 'Institucion' : 'Persona'} · {timeAgo(id.created_at)}
                      </p>
                    </div>
                    <span className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                      id.status === 'active' ? active ? 'bg-white' : 'bg-emerald-500'
                        : id.status === 'revoked' ? 'bg-red-500' : 'bg-amber-500'
                    }`} />
                  </button>
                )
              })
            )}
          </div>
        </div>

        {/* Documents panel */}
        <div className="bg-white border border-neutral-200 rounded-xl px-5 py-4">
          {!selected ? (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="w-14 h-14 rounded-full bg-neutral-100 flex items-center justify-center mb-3">
                <svg className="w-6 h-6 text-neutral-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
                </svg>
              </div>
              <p className="text-sm font-medium text-neutral-600">Selecciona una identidad</p>
              <p className="text-xs text-neutral-400 mt-1">Veras los documentos firmados, contratos y certificados asociados</p>
            </div>
          ) : (
            <div key={selected.did} className="animate-fade-in">
              {/* Identity header */}
              <div className="flex items-center gap-3 mb-4 pb-3 border-b border-neutral-100">
                <span className={`w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold ${
                  isOrg(selected.did) ? 'bg-main-100 text-main-600' : 'bg-emerald-100 text-emerald-600'
                }`}>
                  {initials(extractName(selected.did))}
                </span>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-semibold text-neutral-900">{extractName(selected.did)}</p>
                  <p className="text-[10px] text-neutral-400">
                    {isOrg(selected.did) ? 'Institucion' : 'Persona'} · Firma Electronica Avanzada · PQC
                  </p>
                </div>
                <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${
                  selected.status === 'active' ? 'bg-emerald-100 text-emerald-700' : 'bg-red-100 text-red-700'
                }`}>
                  {selected.status === 'active' ? 'Activa' : 'Inactiva'}
                </span>
              </div>

              {/* Stats */}
              <div className="grid grid-cols-3 gap-3 mb-4">
                <div className="bg-neutral-50 rounded-lg px-3 py-2">
                  <p className="text-[9px] text-neutral-400 uppercase">Documentos</p>
                  <p className="text-lg font-bold text-neutral-800">{docs.length}</p>
                </div>
                <div className="bg-neutral-50 rounded-lg px-3 py-2">
                  <p className="text-[9px] text-neutral-400 uppercase">Vigentes</p>
                  <p className="text-lg font-bold text-emerald-600">{docs.filter((d) => docStatus(d) === 'vigente').length}</p>
                </div>
                <div className="bg-neutral-50 rounded-lg px-3 py-2">
                  <p className="text-[9px] text-neutral-400 uppercase">Firmados</p>
                  <p className="text-lg font-bold text-main-600">{docs.length}</p>
                </div>
              </div>

              {/* Document list */}
              <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">
                Documentos firmados
              </p>

              {docsLoading ? (
                <p className="text-xs text-neutral-400">Cargando documentos...</p>
              ) : docs.length === 0 ? (
                <p className="text-xs text-neutral-400 py-4">Esta identidad aun no tiene documentos firmados.</p>
              ) : (
                <div className="space-y-2">
                  {docs.map((doc) => {
                    const status = docStatus(doc)
                    const issuer = extractName(doc.issuer_did)
                    return (
                      <button
                        key={doc.id}
                        onClick={() => setOpenDoc(doc)}
                        className="w-full text-left bg-neutral-50 border border-neutral-200 rounded-lg px-4 py-3 hover:border-main-300 hover:bg-main-50/30 transition-colors"
                      >
                        <div className="flex items-start justify-between">
                          <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium text-neutral-800">{doc.cred_type}</p>
                            <p className="text-[10px] text-neutral-500 mt-0.5">
                              Firmado por {issuer} · {fmtDate(doc.issued_at)}
                            </p>
                          </div>
                          <span className={`text-[9px] px-1.5 py-0.5 rounded font-medium flex-shrink-0 ml-2 ${DOC_STATUS_STYLES[status]}`}>
                            {status.charAt(0).toUpperCase() + status.slice(1)}
                          </span>
                        </div>
                        {doc.claims && Object.keys(doc.claims).length > 0 && (
                          <div className="flex gap-3 mt-2">
                            {Object.entries(doc.claims).slice(0, 3).map(([k, v]) => (
                              <span key={k} className="text-[10px] text-neutral-400">
                                <span className="text-neutral-500">{k}:</span> {String(v)}
                              </span>
                            ))}
                          </div>
                        )}
                      </button>
                    )
                  })}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Document detail drawer */}
      {openDoc && (
        <>
          <div className="fixed inset-0 z-50 bg-black/20 animate-backdrop" onClick={() => setOpenDoc(null)} />
          <div className="fixed inset-y-0 right-0 z-50 w-full max-w-md bg-white shadow-xl flex flex-col animate-slide-in">
            {/* Drawer header */}
            <div className="px-5 py-4 border-b border-neutral-200">
              <div className="flex items-center justify-between">
                <p className="text-base font-bold text-neutral-900">{openDoc.cred_type}</p>
                <button onClick={() => setOpenDoc(null)} className="p-1.5 rounded-lg hover:bg-neutral-100">
                  <svg className="w-5 h-5 text-neutral-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <p className="text-xs text-neutral-400 mt-0.5">Documento con firma electronica avanzada</p>
            </div>

            {/* Drawer content */}
            <div className="flex-1 overflow-y-auto px-5 py-4">
              {/* Status banner */}
              {(() => {
                const status = docStatus(openDoc)
                return (
                  <div className={`rounded-lg px-4 py-2.5 mb-4 ${DOC_STATUS_STYLES[status]} bg-opacity-50`}>
                    <p className="text-xs font-medium">
                      {status === 'vigente' ? 'Documento vigente — firma electronica valida'
                        : status === 'expirado' ? 'Documento expirado'
                        : 'Documento revocado — firma invalidada'}
                    </p>
                  </div>
                )
              })()}

              {/* Document fields */}
              <div className="space-y-3 mb-6">
                <Field label="Tipo de documento" value={openDoc.cred_type} />
                <Field label="Firmado por" value={extractName(openDoc.issuer_did)} sub={openDoc.issuer_did} />
                <Field label="Titular" value={extractName(openDoc.subject_did)} sub={openDoc.subject_did} />
                <Field label="Fecha de firma" value={fmtDate(openDoc.issued_at)} />
                {openDoc.expires_at > 0 && <Field label="Vencimiento" value={fmtDate(openDoc.expires_at)} />}
                {openDoc.revoked_at && <Field label="Revocado" value={fmtDate(openDoc.revoked_at)} />}
              </div>

              {/* Claims / content */}
              {openDoc.claims && Object.keys(openDoc.claims).length > 0 && (
                <div className="mb-6">
                  <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Contenido del documento</p>
                  <div className="bg-neutral-50 border border-neutral-200 rounded-lg p-3 space-y-2">
                    {Object.entries(openDoc.claims).map(([k, v]) => (
                      <div key={k} className="flex justify-between items-baseline">
                        <span className="text-xs text-neutral-500 capitalize">{k.replace(/_/g, ' ')}</span>
                        <span className="text-xs text-neutral-800 font-medium text-right">{String(v)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Cryptographic proof */}
              <div>
                <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-2">Garantia criptografica</p>
                <div className="bg-neutral-900 rounded-lg p-3 space-y-1.5">
                  <ProofRow label="Algoritmo" value="ML-DSA-65 (FIPS 204)" />
                  <ProofRow label="ID documento" value={openDoc.id} />
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
      <span className="text-[10px] text-emerald-400 font-mono truncate ml-2 max-w-[200px]">{value}</span>
    </div>
  )
}
