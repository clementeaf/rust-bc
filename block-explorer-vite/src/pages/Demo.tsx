import { useState, Fragment } from 'react'
import PageIntro from '../components/PageIntro'
import {
  createIdentity,
  createCredential,
  getCredential,
  getCredentialsBySubject,
  type IdentityRecord,
  type Credential,
} from '../lib/api'
import { fmtDate } from '../lib/format'

type Step = 1 | 2 | 3 | 4 | 5

interface StepState {
  loading: boolean
  done: boolean
  error: string
}

const INITIAL: StepState = { loading: false, done: false, error: '' }

export default function Demo() {
  const [step, setStep] = useState<Step>(1)

  const [issuerName, setIssuerName] = useState('Universidad de Chile')
  const [issuerDid, setIssuerDid] = useState('')
  const [s1, setS1] = useState<StepState>(INITIAL)
  const [issuerRecord, setIssuerRecord] = useState<IdentityRecord | null>(null)

  const [candidateName, setCandidateName] = useState('Juan Perez')
  const [s2, setS2] = useState<StepState>(INITIAL)
  const [candidateDid, setCandidateDid] = useState('')
  const [candidateRecord, setCandidateRecord] = useState<IdentityRecord | null>(null)

  const [credType, setCredType] = useState('Titulo Ingenieria Civil Informatica')
  const [s3, setS3] = useState<StepState>(INITIAL)
  const [credential, setCredential] = useState<Credential | null>(null)

  const [s4, setS4] = useState<StepState>(INITIAL)
  const [verified, setVerified] = useState<Credential | null>(null)
  const [verifyTime, setVerifyTime] = useState<number>(0)

  const [s5, setS5] = useState<StepState>(INITIAL)
  const [allCreds, setAllCreds] = useState<Credential[]>([])

  const states = [s1, s2, s3, s4, s5]

  // ── Handlers ────────────────────────────────────────────────────────────────

  const handleRegisterIssuer = async () => {
    setS1({ loading: true, done: false, error: '' })
    try {
      const did = `did:cerulean:${issuerName.toLowerCase().replace(/\s+/g, '-')}`
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
      const did = `did:cerulean:${candidateName.toLowerCase().replace(/\s+/g, '-')}`
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
      const cred = await createCredential(id, issuerDid, candidateDid, credType, now, 0)
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
    setS1(INITIAL); setS2(INITIAL); setS3(INITIAL); setS4(INITIAL); setS5(INITIAL)
    setIssuerDid(''); setIssuerRecord(null)
    setCandidateDid(''); setCandidateRecord(null)
    setCredential(null); setVerified(null); setVerifyTime(0); setAllCreds([])
    setIssuerName('Universidad de Chile')
    setCandidateName('Juan Perez')
    setCredType('Titulo Ingenieria Civil Informatica')
  }

  // ── Render ──────────────────────────────────────────────────────────────────

  const stepLabels = ['Registrar Emisor', 'Registrar Candidato', 'Emitir Titulo', 'Verificar', 'Perfil Completo']

  return (
    <>
      <PageIntro title="Verificacion de Credenciales RRHH">
        Flujo en 5 pasos: una institucion emite un titulo, un candidato lo presenta, y una empresa lo verifica en milisegundos.
      </PageIntro>

      {/* Step indicator — horizontal, compact */}
      <div className="flex items-center gap-1.5 mb-5">
        {stepLabels.map((label, i) => {
          const n = (i + 1) as Step
          const done = states[i].done
          const active = n === step
          return (
            <Fragment key={n}>
              <button
                onClick={() => (done || n <= step) ? setStep(n) : undefined}
                className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium transition-colors ${
                  done ? 'bg-emerald-100 text-emerald-700' : active ? 'bg-main-500 text-white' : 'bg-neutral-100 text-neutral-400'
                }`}
              >
                <span className={`w-4 h-4 rounded-full flex items-center justify-center text-[9px] font-bold ${
                  done ? 'bg-emerald-500 text-white' : active ? 'bg-white/25' : ''
                }`}>
                  {done ? '✓' : n}
                </span>
                <span className="hidden sm:inline">{label}</span>
              </button>
              {i < 4 && <div className="w-4 h-px bg-neutral-200 hidden sm:block" />}
            </Fragment>
          )
        })}
        <button onClick={handleReset} className="ml-auto text-xs text-neutral-400 hover:text-neutral-600">Reiniciar</button>
      </div>

      {/* Active step — max-width for readability */}
      <div className="max-w-2xl">
        <div className="bg-white border border-neutral-200 rounded-xl p-5">

          {/* Step 1 */}
          {step === 1 && (
            <>
              <StepHeader n={1} title="Registrar institucion emisora" />
              <p className="text-sm text-neutral-500 mb-4">
                La universidad se registra como identidad descentralizada (DID) en la red. Esto la habilita para emitir credenciales verificables.
              </p>
              {!s1.done && (
                <div className="flex gap-3">
                  <input value={issuerName} onChange={(e) => setIssuerName(e.target.value)}
                    placeholder="Nombre de la institucion"
                    className="flex-1 border border-neutral-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500" />
                  <Btn onClick={handleRegisterIssuer} loading={s1.loading} disabled={!issuerName.trim()} label="Registrar DID" />
                </div>
              )}
              <StepError error={s1.error} />
              {s1.done && issuerRecord && (
                <ResultBox label="Emisor registrado" fields={[
                  ['DID', issuerRecord.did], ['Status', issuerRecord.status], ['Registrado', fmtDate(issuerRecord.created_at)],
                ]} />
              )}
            </>
          )}

          {/* Step 2 */}
          {step === 2 && (
            <>
              <StepHeader n={2} title="Registrar candidato" />
              <p className="text-sm text-neutral-500 mb-4">
                El candidato obtiene su identidad digital soberana. En produccion se vincularia a ClaveUnica u otro proveedor.
              </p>
              {!s2.done && (
                <div className="flex gap-3">
                  <input value={candidateName} onChange={(e) => setCandidateName(e.target.value)}
                    placeholder="Nombre del candidato"
                    className="flex-1 border border-neutral-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500" />
                  <Btn onClick={handleRegisterCandidate} loading={s2.loading} disabled={!candidateName.trim()} label="Registrar DID" />
                </div>
              )}
              <StepError error={s2.error} />
              {s2.done && candidateRecord && (
                <ResultBox label="Candidato registrado" fields={[
                  ['DID', candidateRecord.did], ['Status', candidateRecord.status], ['Registrado', fmtDate(candidateRecord.created_at)],
                ]} />
              )}
            </>
          )}

          {/* Step 3 */}
          {step === 3 && (
            <>
              <StepHeader n={3} title="Emitir titulo / credencial" />
              <p className="text-sm text-neutral-500 mb-4">
                La institucion emite una credencial verificable al candidato, firmada criptograficamente y registrada en la blockchain.
              </p>
              {!s3.done && (
                <div className="flex gap-3">
                  <input value={credType} onChange={(e) => setCredType(e.target.value)}
                    placeholder="Tipo de credencial"
                    className="flex-1 border border-neutral-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-main-500" />
                  <Btn onClick={handleIssueCredential} loading={s3.loading} disabled={!credType.trim()} label="Emitir" />
                </div>
              )}
              <StepError error={s3.error} />
              {s3.done && credential && (
                <ResultBox label="Credencial emitida" fields={[
                  ['ID', credential.id], ['Emisor', credential.issuer_did], ['Titular', credential.subject_did],
                  ['Tipo', credential.cred_type], ['Fecha', fmtDate(credential.issued_at)],
                ]} />
              )}
            </>
          )}

          {/* Step 4 */}
          {step === 4 && (
            <>
              <StepHeader n={4} title="Empresa verifica credencial" />
              <p className="text-sm text-neutral-500 mb-4">
                Verificacion en tiempo real. Sin llamar a la universidad, sin esperar dias. Prueba criptografica inmutable.
              </p>
              {!s4.done && <Btn onClick={handleVerify} loading={s4.loading} label="Verificar credencial" />}
              <StepError error={s4.error} />
              {s4.done && verified && (
                <>
                  <ResultBox label="Credencial verificada" badge={`${verifyTime}ms`} fields={[
                    ['Titulo', verified.cred_type], ['Emisor', verified.issuer_did], ['Titular', verified.subject_did],
                    ['Fecha', fmtDate(verified.issued_at)], ['Revocada', verified.revoked_at ? `Si (${fmtDate(verified.revoked_at)})` : 'No'],
                  ]} />
                  <div className="mt-3 bg-emerald-100 rounded-lg px-4 py-2.5 text-sm text-emerald-800">
                    <span className="font-semibold">Tradicional:</span> 3-15 dias habiles · <span className="font-semibold">Blockchain:</span> {verifyTime}ms
                  </div>
                </>
              )}
            </>
          )}

          {/* Step 5 */}
          {step === 5 && (
            <>
              <StepHeader n={5} title="Perfil completo del candidato" />
              <p className="text-sm text-neutral-500 mb-4">
                Todas las credenciales asociadas al DID del candidato en una sola consulta.
              </p>
              {!s5.done && <Btn onClick={handleFullProfile} loading={s5.loading} label="Ver perfil completo" />}
              <StepError error={s5.error} />
              {s5.done && (
                <div className="bg-emerald-50 border border-emerald-200 rounded-lg p-3">
                  <p className="text-sm text-emerald-700 font-medium mb-2">
                    {allCreds.length} credencial{allCreds.length !== 1 ? 'es' : ''} para {candidateDid}
                  </p>
                  {allCreds.map((c) => (
                    <div key={c.id} className="flex items-center justify-between py-2 border-b border-emerald-100 last:border-0">
                      <div>
                        <p className="text-sm font-medium text-neutral-800">{c.cred_type}</p>
                        <p className="text-xs text-neutral-400 font-mono">{c.issuer_did} · {fmtDate(c.issued_at)}</p>
                      </div>
                      <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                        c.revoked_at ? 'bg-red-100 text-red-700' : 'bg-emerald-200 text-emerald-800'
                      }`}>
                        {c.revoked_at ? 'Revocada' : 'Valida'}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
        </div>

        {/* Summary — after step 5 done */}
        {s5.done && (
          <div className="bg-neutral-900 text-white rounded-xl p-5 mt-4">
            <h3 className="text-sm font-bold mb-3">Resumen</h3>
            <div className="grid grid-cols-3 gap-4">
              <div>
                <p className="text-xs text-neutral-400 uppercase">Verificacion</p>
                <p className="text-2xl font-bold text-emerald-400">{verifyTime}ms</p>
                <p className="text-xs text-neutral-500">vs 3-15 dias</p>
              </div>
              <div>
                <p className="text-xs text-neutral-400 uppercase">Credenciales</p>
                <p className="text-2xl font-bold">{allCreds.length}</p>
                <p className="text-xs text-neutral-500">en 1 consulta</p>
              </div>
              <div>
                <p className="text-xs text-neutral-400 uppercase">Seguridad</p>
                <p className="text-2xl font-bold">PQC</p>
                <p className="text-xs text-neutral-500">ML-DSA-65</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </>
  )
}

// ── Shared ───────────────────────────────────────────────────────────────────

function StepHeader({ n, title }: { n: number; title: string }) {
  return (
    <div className="flex items-center gap-2 mb-2">
      <span className="w-6 h-6 rounded-full bg-main-500 text-white flex items-center justify-center text-xs font-bold">{n}</span>
      <h2 className="text-base font-semibold text-neutral-900">{title}</h2>
    </div>
  )
}

function Btn({ onClick, loading, disabled, label }: { onClick: () => void; loading: boolean; disabled?: boolean; label: string }) {
  return (
    <button onClick={onClick} disabled={loading || disabled}
      className="bg-main-500 text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-main-600 disabled:opacity-50 transition-colors">
      {loading ? 'Procesando...' : label}
    </button>
  )
}

function StepError({ error }: { error: string }) {
  return error ? <p className="text-sm text-red-500 mt-2">{error}</p> : null
}

function ResultBox({ label, badge, fields }: { label: string; badge?: string; fields: [string, string][] }) {
  return (
    <div className="bg-emerald-50 border border-emerald-200 rounded-lg p-4 mt-3">
      <div className="flex items-center justify-between mb-2">
        <p className="text-sm font-medium text-emerald-800">{label}</p>
        {badge && <span className="bg-emerald-200 text-emerald-800 text-xs font-bold px-2 py-0.5 rounded-full">{badge}</span>}
      </div>
      <dl className="grid grid-cols-[100px_1fr] gap-x-3 gap-y-1 text-sm">
        {fields.map(([k, v]) => (
          <Fragment key={k}>
            <dt className="text-emerald-600">{k}</dt>
            <dd className="text-emerald-800 font-mono truncate">{v}</dd>
          </Fragment>
        ))}
      </dl>
    </div>
  )
}
