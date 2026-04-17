import { useState } from 'react'
import PageIntro from '../components/PageIntro'
import {
  createIdentity,
  getIdentity,
  createCredential,
  getCredential,
  getCredentialsBySubject,
  type IdentityRecord,
  type Credential,
} from '../lib/api'

type Step = 1 | 2 | 3 | 4 | 5

interface StepState {
  loading: boolean
  done: boolean
  error: string
}

const INITIAL: StepState = { loading: false, done: false, error: '' }

function fmtDate(ts: number) {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleDateString('es-CL', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

export default function Demo() {
  const [step, setStep] = useState<Step>(1)

  // Step 1 — Issuer (university)
  const [issuerName, setIssuerName] = useState('Universidad de Chile')
  const [issuerDid, setIssuerDid] = useState('')
  const [s1, setS1] = useState<StepState>(INITIAL)
  const [issuerRecord, setIssuerRecord] = useState<IdentityRecord | null>(null)

  // Step 2 — Subject (candidate)
  const [candidateName, setCandidateName] = useState('Juan Perez')
  const [s2, setS2] = useState<StepState>(INITIAL)
  const [candidateDid, setCandidateDid] = useState('')
  const [candidateRecord, setCandidateRecord] = useState<IdentityRecord | null>(null)

  // Step 3 — Issue credential
  const [credType, setCredType] = useState('Titulo Ingenieria Civil Informatica')
  const [s3, setS3] = useState<StepState>(INITIAL)
  const [credential, setCredential] = useState<Credential | null>(null)

  // Step 4 — Verify credential
  const [s4, setS4] = useState<StepState>(INITIAL)
  const [verified, setVerified] = useState<Credential | null>(null)
  const [verifyTime, setVerifyTime] = useState<number>(0)

  // Step 5 — Full profile
  const [s5, setS5] = useState<StepState>(INITIAL)
  const [allCreds, setAllCreds] = useState<Credential[]>([])

  // ── Step handlers ──────────────────────────────────────────────────────────

  const handleRegisterIssuer = async () => {
    setS1({ loading: true, done: false, error: '' })
    try {
      const did = `did:rustbc:${issuerName.toLowerCase().replace(/\s+/g, '-')}`
      const record = await createIdentity(did, 'active')
      setIssuerDid(did)
      setIssuerRecord(record)
      setS1({ loading: false, done: true, error: '' })
      setStep(2)
    } catch (err) {
      setS1({ loading: false, done: false, error: err instanceof Error ? err.message : 'Error' })
    }
  }

  const handleRegisterCandidate = async () => {
    setS2({ loading: true, done: false, error: '' })
    try {
      const did = `did:rustbc:${candidateName.toLowerCase().replace(/\s+/g, '-')}`
      const record = await createIdentity(did, 'active')
      setCandidateDid(did)
      setCandidateRecord(record)
      setS2({ loading: false, done: true, error: '' })
      setStep(3)
    } catch (err) {
      setS2({ loading: false, done: false, error: err instanceof Error ? err.message : 'Error' })
    }
  }

  const handleIssueCredential = async () => {
    setS3({ loading: true, done: false, error: '' })
    try {
      const now = Math.floor(Date.now() / 1000)
      const id = `cred-${Date.now()}`
      const cred = await createCredential(
        id,
        issuerDid,
        candidateDid,
        credType,
        now,
        0, // no expiry
      )
      setCredential(cred)
      setS3({ loading: false, done: true, error: '' })
      setStep(4)
    } catch (err) {
      setS3({ loading: false, done: false, error: err instanceof Error ? err.message : 'Error' })
    }
  }

  const handleVerify = async () => {
    if (!credential) return
    setS4({ loading: true, done: false, error: '' })
    try {
      const t0 = performance.now()
      const result = await getCredential(credential.id)
      const elapsed = performance.now() - t0
      setVerified(result)
      setVerifyTime(Math.round(elapsed))
      setS4({ loading: false, done: true, error: '' })
      setStep(5)
    } catch (err) {
      setS4({ loading: false, done: false, error: err instanceof Error ? err.message : 'Error' })
    }
  }

  const handleFullProfile = async () => {
    setS5({ loading: true, done: false, error: '' })
    try {
      const creds = await getCredentialsBySubject(candidateDid)
      setAllCreds(creds)
      setS5({ loading: false, done: true, error: '' })
    } catch (err) {
      setS5({ loading: false, done: false, error: err instanceof Error ? err.message : 'Error' })
    }
  }

  const handleReset = () => {
    setStep(1)
    setS1(INITIAL)
    setS2(INITIAL)
    setS3(INITIAL)
    setS4(INITIAL)
    setS5(INITIAL)
    setIssuerDid('')
    setIssuerRecord(null)
    setCandidateDid('')
    setCandidateRecord(null)
    setCredential(null)
    setVerified(null)
    setVerifyTime(0)
    setAllCreds([])
    setIssuerName('Universidad de Chile')
    setCandidateName('Juan Perez')
    setCredType('Titulo Ingenieria Civil Informatica')
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  const stepClass = (s: Step) =>
    s < step || (s === step && eval(`s${s}`).done)
      ? 'bg-green-500 text-white'
      : s === step
        ? 'bg-main-500 text-white'
        : 'bg-neutral-200 text-neutral-500'

  const isStepDone = (s: Step) => {
    const states = [s1, s2, s3, s4, s5]
    return states[s - 1].done
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <>
      <PageIntro title="Demo: Verificacion de Credenciales para RRHH">
        Flujo completo en 5 pasos: una institucion emite un titulo, un candidato lo presenta, y una
        empresa lo verifica en segundos con prueba criptografica inmutable.
      </PageIntro>

      {/* Step indicator */}
      <div className="flex items-center gap-2 mb-8 flex-wrap">
        {[
          { n: 1 as Step, label: 'Registrar Emisor' },
          { n: 2 as Step, label: 'Registrar Candidato' },
          { n: 3 as Step, label: 'Emitir Titulo' },
          { n: 4 as Step, label: 'Verificar' },
          { n: 5 as Step, label: 'Perfil Completo' },
        ].map(({ n, label }, i) => (
          <div key={n} className="flex items-center gap-2">
            <div
              className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold ${
                isStepDone(n)
                  ? 'bg-green-500 text-white'
                  : n === step
                    ? 'bg-main-500 text-white'
                    : 'bg-neutral-200 text-neutral-500'
              }`}
            >
              {isStepDone(n) ? '✓' : n}
            </div>
            <span
              className={`text-xs font-medium hidden sm:inline ${
                n === step ? 'text-neutral-900' : 'text-neutral-400'
              }`}
            >
              {label}
            </span>
            {i < 4 && (
              <div className="w-6 h-px bg-neutral-300 hidden sm:block" />
            )}
          </div>
        ))}
        <button
          onClick={handleReset}
          className="ml-auto text-neutral-400 hover:text-neutral-600 text-xs font-medium"
        >
          Reiniciar demo
        </button>
      </div>

      {/* ── Step 1: Register Issuer ─────────────────────────────────────────── */}
      <div className={`bg-white border rounded-2xl p-5 mb-4 transition-all ${step === 1 ? 'border-main-300 shadow-sm' : 'border-neutral-200'}`}>
        <div className="flex items-center gap-3 mb-3">
          <div className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${stepClass(1)}`}>
            {s1.done ? '✓' : '1'}
          </div>
          <h2 className="text-lg font-semibold text-neutral-900">Registrar institucion emisora</h2>
        </div>
        <p className="text-neutral-500 text-sm mb-4">
          La universidad o certificadora se registra como identidad descentralizada (DID) en la red.
          Esto la habilita para emitir credenciales verificables.
        </p>
        {step === 1 && !s1.done && (
          <div className="flex flex-col sm:flex-row gap-3">
            <input
              type="text"
              value={issuerName}
              onChange={(e) => setIssuerName(e.target.value)}
              placeholder="Nombre de la institucion"
              className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm
                         focus:outline-none focus:ring-2 focus:ring-main-500"
            />
            <button
              onClick={handleRegisterIssuer}
              disabled={s1.loading || !issuerName.trim()}
              className="bg-main-500 text-white px-5 py-2 rounded-xl text-sm font-medium
                         hover:bg-main-600 disabled:opacity-50 transition-colors"
            >
              {s1.loading ? 'Registrando...' : 'Registrar como DID'}
            </button>
          </div>
        )}
        {s1.error && <p className="text-red-500 text-sm mt-2">{s1.error}</p>}
        {s1.done && issuerRecord && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-3 mt-2">
            <p className="text-green-800 text-sm font-medium">Emisor registrado</p>
            <p className="text-green-700 text-xs font-mono mt-1">{issuerRecord.did}</p>
            <p className="text-green-600 text-xs mt-1">Status: {issuerRecord.status} · Registrado: {fmtDate(issuerRecord.created_at)}</p>
          </div>
        )}
      </div>

      {/* ── Step 2: Register Candidate ──────────────────────────────────────── */}
      <div className={`bg-white border rounded-2xl p-5 mb-4 transition-all ${step === 2 ? 'border-main-300 shadow-sm' : 'border-neutral-200'}`}>
        <div className="flex items-center gap-3 mb-3">
          <div className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${stepClass(2)}`}>
            {s2.done ? '✓' : '2'}
          </div>
          <h2 className="text-lg font-semibold text-neutral-900">Registrar candidato</h2>
        </div>
        <p className="text-neutral-500 text-sm mb-4">
          El candidato obtiene su identidad digital. En produccion esto se vincularia a ClaveUnica u otro
          proveedor de identidad.
        </p>
        {step === 2 && !s2.done && (
          <div className="flex flex-col sm:flex-row gap-3">
            <input
              type="text"
              value={candidateName}
              onChange={(e) => setCandidateName(e.target.value)}
              placeholder="Nombre del candidato"
              className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm
                         focus:outline-none focus:ring-2 focus:ring-main-500"
            />
            <button
              onClick={handleRegisterCandidate}
              disabled={s2.loading || !candidateName.trim()}
              className="bg-main-500 text-white px-5 py-2 rounded-xl text-sm font-medium
                         hover:bg-main-600 disabled:opacity-50 transition-colors"
            >
              {s2.loading ? 'Registrando...' : 'Registrar como DID'}
            </button>
          </div>
        )}
        {s2.error && <p className="text-red-500 text-sm mt-2">{s2.error}</p>}
        {s2.done && candidateRecord && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-3 mt-2">
            <p className="text-green-800 text-sm font-medium">Candidato registrado</p>
            <p className="text-green-700 text-xs font-mono mt-1">{candidateRecord.did}</p>
            <p className="text-green-600 text-xs mt-1">Status: {candidateRecord.status} · Registrado: {fmtDate(candidateRecord.created_at)}</p>
          </div>
        )}
      </div>

      {/* ── Step 3: Issue Credential ────────────────────────────────────────── */}
      <div className={`bg-white border rounded-2xl p-5 mb-4 transition-all ${step === 3 ? 'border-main-300 shadow-sm' : 'border-neutral-200'}`}>
        <div className="flex items-center gap-3 mb-3">
          <div className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${stepClass(3)}`}>
            {s3.done ? '✓' : '3'}
          </div>
          <h2 className="text-lg font-semibold text-neutral-900">Emitir titulo / credencial</h2>
        </div>
        <p className="text-neutral-500 text-sm mb-4">
          La institucion emite una credencial verificable al candidato. Queda firmada criptograficamente
          y registrada de forma inmutable en la blockchain.
        </p>
        {step === 3 && !s3.done && (
          <div className="flex flex-col sm:flex-row gap-3">
            <input
              type="text"
              value={credType}
              onChange={(e) => setCredType(e.target.value)}
              placeholder="Tipo de credencial"
              className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm
                         focus:outline-none focus:ring-2 focus:ring-main-500"
            />
            <button
              onClick={handleIssueCredential}
              disabled={s3.loading || !credType.trim()}
              className="bg-main-500 text-white px-5 py-2 rounded-xl text-sm font-medium
                         hover:bg-main-600 disabled:opacity-50 transition-colors"
            >
              {s3.loading ? 'Emitiendo...' : 'Emitir credencial'}
            </button>
          </div>
        )}
        {s3.error && <p className="text-red-500 text-sm mt-2">{s3.error}</p>}
        {s3.done && credential && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-3 mt-2">
            <p className="text-green-800 text-sm font-medium">Credencial emitida</p>
            <dl className="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-1 text-xs mt-2">
              <dt className="text-green-600">ID</dt>
              <dd className="text-green-800 font-mono">{credential.id}</dd>
              <dt className="text-green-600">Emisor</dt>
              <dd className="text-green-800 font-mono">{credential.issuer_did}</dd>
              <dt className="text-green-600">Titular</dt>
              <dd className="text-green-800 font-mono">{credential.subject_did}</dd>
              <dt className="text-green-600">Tipo</dt>
              <dd className="text-green-800">{credential.cred_type}</dd>
              <dt className="text-green-600">Fecha emision</dt>
              <dd className="text-green-800">{fmtDate(credential.issued_at)}</dd>
              <dt className="text-green-600">Revocada</dt>
              <dd className="text-green-800">{credential.revoked_at ? 'Si' : 'No'}</dd>
            </dl>
          </div>
        )}
      </div>

      {/* ── Step 4: Verify ──────────────────────────────────────────────────── */}
      <div className={`bg-white border rounded-2xl p-5 mb-4 transition-all ${step === 4 ? 'border-main-300 shadow-sm' : 'border-neutral-200'}`}>
        <div className="flex items-center gap-3 mb-3">
          <div className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${stepClass(4)}`}>
            {s4.done ? '✓' : '4'}
          </div>
          <h2 className="text-lg font-semibold text-neutral-900">Empresa verifica credencial</h2>
        </div>
        <p className="text-neutral-500 text-sm mb-4">
          La empresa contratante verifica la credencial en tiempo real. Sin llamar a la universidad,
          sin esperar dias. Prueba criptografica inmutable.
        </p>
        {step === 4 && !s4.done && (
          <button
            onClick={handleVerify}
            disabled={s4.loading}
            className="bg-main-500 text-white px-5 py-2 rounded-xl text-sm font-medium
                       hover:bg-main-600 disabled:opacity-50 transition-colors"
          >
            {s4.loading ? 'Verificando...' : 'Verificar credencial'}
          </button>
        )}
        {s4.error && <p className="text-red-500 text-sm mt-2">{s4.error}</p>}
        {s4.done && verified && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-3 mt-2">
            <div className="flex items-center justify-between mb-2">
              <p className="text-green-800 text-sm font-medium">Credencial verificada</p>
              <span className="bg-green-200 text-green-800 text-xs font-bold px-2 py-0.5 rounded-full">
                {verifyTime}ms
              </span>
            </div>
            <dl className="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-1 text-xs">
              <dt className="text-green-600">Titulo</dt>
              <dd className="text-green-800">{verified.cred_type}</dd>
              <dt className="text-green-600">Emitido por</dt>
              <dd className="text-green-800 font-mono">{verified.issuer_did}</dd>
              <dt className="text-green-600">Titular</dt>
              <dd className="text-green-800 font-mono">{verified.subject_did}</dd>
              <dt className="text-green-600">Fecha</dt>
              <dd className="text-green-800">{fmtDate(verified.issued_at)}</dd>
              <dt className="text-green-600">Expirada</dt>
              <dd className="text-green-800">{verified.expires_at ? fmtDate(verified.expires_at) : 'No expira'}</dd>
              <dt className="text-green-600">Revocada</dt>
              <dd className="text-green-800">{verified.revoked_at ? `Si (${fmtDate(verified.revoked_at)})` : 'No'}</dd>
            </dl>
            <div className="mt-3 bg-green-100 rounded-lg p-2">
              <p className="text-green-800 text-xs">
                <span className="font-bold">Proceso tradicional:</span> 3-15 dias habiles contactando a la universidad.
              </p>
              <p className="text-green-800 text-xs">
                <span className="font-bold">Con blockchain:</span> {verifyTime}ms con prueba criptografica inmutable.
              </p>
            </div>
          </div>
        )}
      </div>

      {/* ── Step 5: Full profile ────────────────────────────────────────────── */}
      <div className={`bg-white border rounded-2xl p-5 mb-4 transition-all ${step === 5 ? 'border-main-300 shadow-sm' : 'border-neutral-200'}`}>
        <div className="flex items-center gap-3 mb-3">
          <div className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${stepClass(5)}`}>
            {s5.done ? '✓' : '5'}
          </div>
          <h2 className="text-lg font-semibold text-neutral-900">Perfil completo del candidato</h2>
        </div>
        <p className="text-neutral-500 text-sm mb-4">
          Consultar todas las credenciales asociadas al DID del candidato: titulos, certificaciones,
          antecedentes laborales — todo en una sola consulta.
        </p>
        {step === 5 && !s5.done && (
          <button
            onClick={handleFullProfile}
            disabled={s5.loading}
            className="bg-main-500 text-white px-5 py-2 rounded-xl text-sm font-medium
                       hover:bg-main-600 disabled:opacity-50 transition-colors"
          >
            {s5.loading ? 'Consultando...' : 'Ver perfil completo'}
          </button>
        )}
        {s5.error && <p className="text-red-500 text-sm mt-2">{s5.error}</p>}
        {s5.done && (
          <div className="bg-green-50 border border-green-200 rounded-xl p-3 mt-2">
            <p className="text-green-800 text-sm font-medium mb-2">
              {allCreds.length} credencial{allCreds.length !== 1 ? 'es' : ''} encontrada{allCreds.length !== 1 ? 's' : ''} para {candidateDid}
            </p>
            {allCreds.map((c) => (
              <div key={c.id} className="bg-white border border-green-200 rounded-lg p-3 mb-2 last:mb-0">
                <div className="flex items-center justify-between">
                  <span className="text-neutral-900 text-sm font-medium">{c.cred_type}</span>
                  {c.revoked_at ? (
                    <span className="bg-red-100 text-red-700 text-xs px-2 py-0.5 rounded-full">Revocada</span>
                  ) : (
                    <span className="bg-green-100 text-green-700 text-xs px-2 py-0.5 rounded-full">Valida</span>
                  )}
                </div>
                <p className="text-neutral-500 text-xs mt-1">
                  Emitida por <span className="font-mono">{c.issuer_did}</span> el {fmtDate(c.issued_at)}
                </p>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* ── Summary box ─────────────────────────────────────────────────────── */}
      {s5.done && (
        <div className="bg-neutral-900 text-white rounded-2xl p-6 mt-6">
          <h2 className="text-lg font-bold mb-3">Resumen de la demo</h2>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
            <div>
              <p className="text-neutral-400 text-xs uppercase tracking-wide">Tiempo verificacion</p>
              <p className="text-2xl font-bold text-green-400 mt-1">{verifyTime}ms</p>
              <p className="text-neutral-500 text-xs mt-1">vs 3-15 dias habiles (manual)</p>
            </div>
            <div>
              <p className="text-neutral-400 text-xs uppercase tracking-wide">Credenciales verificadas</p>
              <p className="text-2xl font-bold text-white mt-1">{allCreds.length}</p>
              <p className="text-neutral-500 text-xs mt-1">en una sola consulta</p>
            </div>
            <div>
              <p className="text-neutral-400 text-xs uppercase tracking-wide">Seguridad</p>
              <p className="text-2xl font-bold text-white mt-1">PQC</p>
              <p className="text-neutral-500 text-xs mt-1">Firma post-cuantica (ML-DSA-65)</p>
            </div>
          </div>
          <div className="mt-4 pt-4 border-t border-neutral-700 text-neutral-400 text-xs">
            Registro inmutable · Sin intermediarios · Revocacion en tiempo real · Canales privados por empresa
          </div>
        </div>
      )}
    </>
  )
}
