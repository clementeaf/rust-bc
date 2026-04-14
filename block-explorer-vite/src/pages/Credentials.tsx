import { useState, type ReactElement } from 'react'
import { Link } from 'react-router-dom'
import {
  createCredential,
  getCredential,
  getCredentialsBySubject,
  type Credential,
} from '../lib/api'

/**
 * Formatea un instante Unix como tiempo relativo en español (pasado).
 * @param ts - Segundos desde epoch
 * @returns Texto tipo "hace 3 d" o "—"
 */
function timeAgo(ts: number): string {
  if (!ts) return '—'
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 0) return 'reciente'
  if (diff < 60) return `hace ${diff}s`
  if (diff < 3600) return `hace ${Math.floor(diff / 60)} min`
  if (diff < 86400) return `hace ${Math.floor(diff / 3600)} h`
  return `hace ${Math.floor(diff / 86400)} d`
}

/**
 * Describe cuándo caduca un registro (futuro o pasado).
 * @param ts - Segundos desde epoch
 * @returns Texto legible en español
 */
function formatExpiry(ts: number): string {
  if (!ts) return '—'
  const now = Math.floor(Date.now() / 1000)
  const diff = ts - now
  if (diff > 0) {
    if (diff < 3600) return `en ${Math.ceil(diff / 60)} min`
    if (diff < 86400) return `en ${Math.ceil(diff / 3600)} h`
    return `en ${Math.ceil(diff / 86400)} d`
  }
  const past = -diff
  if (past < 3600) return `hace ${Math.floor(past / 60)} min`
  if (past < 86400) return `hace ${Math.floor(past / 3600)} h`
  return `hace ${Math.floor(past / 86400)} d`
}

/**
 * Acorta un código largo para tablas.
 * @param d - Código completo
 * @returns Versión truncada
 */
function shortCode(d: string): string {
  return d.length > 24 ? d.slice(0, 12) + '...' + d.slice(-12) : d
}

/**
 * Genera un número interno aleatorio para el registro.
 * @returns Cadena cred-…
 */
function generateCredId(): string {
  const hex = Array.from(crypto.getRandomValues(new Uint8Array(16)))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
  return `cred-${hex}`
}

export default function Credentials(): ReactElement {
  const [credId, setCredId] = useState('')
  const [issuerDid, setIssuerDid] = useState('')
  const [subjectDid, setSubjectDid] = useState('')
  const [credType, setCredType] = useState('acceso')
  const [expiresDays, setExpiresDays] = useState('365')
  const [issueMsg, setIssueMsg] = useState('')
  const [issueErr, setIssueErr] = useState('')

  const [lookupId, setLookupId] = useState('')
  const [credential, setCredential] = useState<Credential | null>(null)
  const [lookupErr, setLookupErr] = useState('')

  const [subjectQuery, setSubjectQuery] = useState('')
  const [subjectCreds, setSubjectCreds] = useState<Credential[]>([])
  const [subjectErr, setSubjectErr] = useState('')

  const handleIssue = async (): Promise<void> => {
    setIssueMsg('')
    setIssueErr('')
    if (!issuerDid.trim() || !subjectDid.trim()) {
      setIssueErr('Falta rellenar quién declara y sobre quién. Los dos códigos deben existir ya en Personas.')
      return
    }
    const now = Math.floor(Date.now() / 1000)
    const expires = now + parseInt(expiresDays || '365', 10) * 86400
    const id = credId.trim() || generateCredId()
    try {
      const cred = await createCredential(
        id,
        issuerDid.trim(),
        subjectDid.trim(),
        credType || 'acceso',
        now,
        expires,
      )
      setIssueMsg('Registro guardado. Puedes copiar el número de documento de abajo para buscarlo después.')
      setCredential(cred)
    } catch (e: unknown) {
      setIssueErr(e instanceof Error ? e.message : 'No se pudo guardar')
    }
  }

  const handleLookup = async (): Promise<void> => {
    setLookupErr('')
    setCredential(null)
    if (!lookupId.trim()) {
      setLookupErr('Escribe el número de documento (empieza por cred-…)')
      return
    }
    try {
      const cred = await getCredential(lookupId.trim())
      setCredential(cred)
    } catch (e: unknown) {
      setLookupErr(e instanceof Error ? e.message : 'No encontramos ese número')
    }
  }

  const handleSubjectSearch = async (): Promise<void> => {
    setSubjectErr('')
    setSubjectCreds([])
    if (!subjectQuery.trim()) {
      setSubjectErr('Pega el código de la persona u organización sobre la que quieres listar registros')
      return
    }
    try {
      const creds = await getCredentialsBySubject(subjectQuery.trim())
      setSubjectCreds(creds)
    } catch (e: unknown) {
      setSubjectErr(e instanceof Error ? e.message : 'No hay resultados')
    }
  }

  return (
    <div className="space-y-8">
      <header className="max-w-3xl">
        <h1 className="text-2xl font-bold text-white tracking-tight">Certificados entre fichas</h1>
        <p className="text-gray-300 text-base mt-3 leading-relaxed">
          Esta pantalla sirve para una sola cosa: <strong className="text-white">dejar por escrito</strong> que{' '}
          <em>una ficha</em> (por ejemplo una empresa) <strong className="text-white">reconoce o autoriza</strong>{' '}
          algo sobre <em>otra ficha</em> (por ejemplo una persona). Es el equivalente digital a un carnet, una
          carta o un vale con fecha de caducidad.
        </p>
      </header>

      <div className="rounded-xl border border-amber-900/50 bg-amber-950/20 p-5 max-w-3xl">
        <p className="font-medium text-amber-100 mb-2">¿Es obligatorio usar esta página?</p>
        <p className="text-sm text-amber-100/90 leading-relaxed">
          <strong className="text-white">No.</strong> Mucha gente solo necesita dar de alta códigos en{' '}
          <Link to="/identity" className="text-cyan-300 hover:text-cyan-200 underline underline-offset-2">
            Personas y organizaciones
          </Link>
          . Solo entra aquí si de verdad quieres guardar <strong className="text-white">un reconocimiento entre dos
          códigos</strong> que ya creaste antes.
        </p>
      </div>

      <div className="rounded-xl border border-gray-700/80 bg-gray-900/50 p-5 text-sm text-gray-300 max-w-3xl space-y-4">
        <p className="font-medium text-white">Qué te pide cada campo</p>
        <ul className="list-disc pl-5 space-y-2 leading-relaxed">
          <li>
            <strong className="text-gray-200">Quién declara</strong>: el código de quien &quot;firma&quot; el
            reconocimiento (empresa, administración, etc.). Debe ser una ficha ya guardada en el nodo.
          </li>
          <li>
            <strong className="text-gray-200">Sobre quién va</strong>: el código de la persona u organización a la
            que afecta el reconocimiento. También debe existir ya como ficha.
          </li>
          <li>
            <strong className="text-gray-200">Qué tipo de reconocimiento es</strong>: una palabra corta que tú
            elijas (por ejemplo &quot;miembro&quot;, &quot;acceso&quot;). Sirve para clasificar el registro.
          </li>
          <li>
            <strong className="text-gray-200">Cuántos días vale</strong>: después de ese plazo el sistema lo
            considera caducado (puedes seguir viéndolo, pero figurará como no vigente).
          </li>
          <li>
            <strong className="text-gray-200">Número de documento (opcional)</strong>: si lo dejas vacío, el
            servidor crea uno automático. Si pones uno, sirve para buscar este registro después.
          </li>
        </ul>
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-white">Crear un reconocimiento nuevo</h2>
        <p className="text-sm text-gray-400 mt-2 mb-6">
          Rellena los dos códigos copiándolos desde la pantalla de Personas o desde donde los tengas anotados.
        </p>

        <div className="grid grid-cols-1 gap-5">
          <div>
            <label htmlFor="issuer-did" className="block text-sm font-medium text-gray-200 mb-1">
              Quién declara (código de la empresa, organismo o persona que emite)
            </label>
            <p className="text-xs text-gray-500 mb-2">Obligatorio. Debe ser una ficha ya creada en el nodo.</p>
            <input
              id="issuer-did"
              value={issuerDid}
              onChange={(e) => setIssuerDid(e.target.value)}
              placeholder="did:bc:…"
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
            />
          </div>
          <div>
            <label htmlFor="subject-did" className="block text-sm font-medium text-gray-200 mb-1">
              Sobre quién va el reconocimiento (código de quien recibe o es el tema)
            </label>
            <p className="text-xs text-gray-500 mb-2">Obligatorio. Otra ficha distinta, también ya creada.</p>
            <input
              id="subject-did"
              value={subjectDid}
              onChange={(e) => setSubjectDid(e.target.value)}
              placeholder="did:bc:…"
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
            />
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
            <div>
              <label htmlFor="cred-type" className="block text-sm font-medium text-gray-200 mb-1">
                Tipo de reconocimiento (una palabra que tú elijas)
              </label>
              <p className="text-xs text-gray-500 mb-2">Ejemplos: acceso, miembro, voluntario.</p>
              <input
                id="cred-type"
                value={credType}
                onChange={(e) => setCredType(e.target.value)}
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm focus:outline-none focus:border-cyan-500"
              />
            </div>
            <div>
              <label htmlFor="expires-days" className="block text-sm font-medium text-gray-200 mb-1">
                Válido durante (días desde hoy)
              </label>
              <p className="text-xs text-gray-500 mb-2">Después se marca como caducado.</p>
              <input
                id="expires-days"
                value={expiresDays}
                onChange={(e) => setExpiresDays(e.target.value)}
                type="number"
                min={1}
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm focus:outline-none focus:border-cyan-500"
              />
            </div>
          </div>
          <div>
            <label htmlFor="cred-id" className="block text-sm font-medium text-gray-200 mb-1">
              Número de documento (opcional)
            </label>
            <p className="text-xs text-gray-500 mb-2">
              Si lo dejas vacío, se genera solo. Si lo rellenas, podrás buscar el registro con ese número.
            </p>
            <div className="flex flex-col sm:flex-row gap-2">
              <input
                id="cred-id"
                value={credId}
                onChange={(e) => setCredId(e.target.value)}
                placeholder="Se rellena solo si quieres un número fijo"
                className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
              />
              <button
                type="button"
                onClick={() => setCredId(generateCredId())}
                className="px-4 py-2.5 text-sm bg-gray-800 text-gray-200 rounded-lg border border-gray-700 hover:bg-gray-700 whitespace-nowrap"
              >
                Generar número
              </button>
            </div>
          </div>
        </div>

        <button
          type="button"
          onClick={() => void handleIssue()}
          className="mt-6 px-5 py-2.5 bg-cyan-600 hover:bg-cyan-500 text-white text-sm font-medium rounded-lg transition-colors"
        >
          Guardar este reconocimiento
        </button>
        {issueMsg && <p className="mt-4 text-sm text-green-400">{issueMsg}</p>}
        {issueErr && <p className="mt-4 text-sm text-red-400">{issueErr}</p>}
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-white mb-1">Buscar un reconocimiento por su número</h2>
        <p className="text-sm text-gray-400 mb-4">
          El número empieza por <code className="text-gray-300">cred-</code> y te lo mostró el sistema al guardar.
        </p>
        <div className="flex flex-col sm:flex-row gap-3">
          <input
            value={lookupId}
            onChange={(e) => setLookupId(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void handleLookup()}
            placeholder="cred-…"
            className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
          />
          <button
            type="button"
            onClick={() => void handleLookup()}
            className="px-5 py-2.5 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium rounded-lg transition-colors"
          >
            Buscar
          </button>
        </div>
        {lookupErr && <p className="mt-4 text-sm text-red-400">{lookupErr}</p>}
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-white mb-1">Ver todos los reconocimientos de una persona</h2>
        <p className="text-sm text-gray-400 mb-4">
          Pega el <strong className="text-gray-300">código de quien recibe</strong> el reconocimiento (la ficha
          &quot;sobre quién&quot;) y listaremos los registros que lo mencionan.
        </p>
        <div className="flex flex-col sm:flex-row gap-3">
          <input
            value={subjectQuery}
            onChange={(e) => setSubjectQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void handleSubjectSearch()}
            placeholder="Código did:bc:… de esa ficha"
            className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
          />
          <button
            type="button"
            onClick={() => void handleSubjectSearch()}
            className="px-5 py-2.5 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium rounded-lg transition-colors"
          >
            Listar
          </button>
        </div>
        {subjectErr && <p className="mt-4 text-sm text-red-400">{subjectErr}</p>}
      </div>

      {credential && (
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
          <h2 className="text-lg font-semibold text-white mb-4">Detalle del registro</h2>
          <dl className="space-y-3 text-sm">
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Número de documento</dt>
              <dd className="text-cyan-400 font-mono break-all">{credential.id}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Quién declara</dt>
              <dd className="text-gray-200 font-mono break-all">{shortCode(credential.issuer_did)}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Sobre quién</dt>
              <dd className="text-gray-200 font-mono break-all">{shortCode(credential.subject_did)}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Tipo</dt>
              <dd>
                <span className="inline-flex px-2 py-0.5 rounded text-xs font-medium bg-gray-800 text-gray-300">
                  {credential.cred_type}
                </span>
              </dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Estado</dt>
              <dd>
                <span
                  className={`inline-flex px-2 py-0.5 rounded text-xs font-medium ${
                    credential.revoked_at
                      ? 'bg-red-900/50 text-red-300'
                      : credential.expires_at && credential.expires_at < Date.now() / 1000
                        ? 'bg-yellow-900/50 text-yellow-300'
                        : 'bg-green-900/50 text-green-300'
                  }`}
                >
                  {credential.revoked_at
                    ? 'Revocado'
                    : credential.expires_at && credential.expires_at < Date.now() / 1000
                      ? 'Caducado'
                      : 'Vigente'}
                </span>
              </dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Emitido</dt>
              <dd className="text-gray-200">{timeAgo(credential.issued_at)}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-44 shrink-0">Caducidad</dt>
              <dd className="text-gray-200">{formatExpiry(credential.expires_at)}</dd>
            </div>
          </dl>
        </div>
      )}

      {subjectCreds.length > 0 && (
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
          <h2 className="text-lg font-semibold text-white mb-4">
            Reconocimientos que mencionan a esta ficha ({subjectCreds.length})
          </h2>
          <p className="text-xs text-gray-500 mb-4">Pulsa una fila para ver el detalle arriba.</p>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-gray-400 text-xs border-b border-gray-800">
                  <th className="text-left py-3 px-2">Número</th>
                  <th className="text-left py-3 px-2">Quién declara</th>
                  <th className="text-left py-3 px-2">Tipo</th>
                  <th className="text-left py-3 px-2">Estado</th>
                  <th className="text-right py-3 px-2">Emitido</th>
                </tr>
              </thead>
              <tbody>
                {subjectCreds.map((c) => (
                  <tr
                    key={c.id}
                    className="border-b border-gray-800/50 hover:bg-gray-900/50 cursor-pointer"
                    onClick={() => {
                      setCredential(c)
                      setLookupId(c.id)
                    }}
                  >
                    <td className="py-3 px-2 text-cyan-400 font-mono text-xs">{shortCode(c.id)}</td>
                    <td className="py-3 px-2 text-gray-300 font-mono text-xs">{shortCode(c.issuer_did)}</td>
                    <td className="py-3 px-2 text-gray-300">{c.cred_type}</td>
                    <td className="py-3 px-2">
                      <span
                        className={`inline-flex px-2 py-0.5 rounded text-xs font-medium ${
                          c.revoked_at
                            ? 'bg-red-900/50 text-red-300'
                            : c.expires_at && c.expires_at < Date.now() / 1000
                              ? 'bg-yellow-900/50 text-yellow-300'
                              : 'bg-green-900/50 text-green-300'
                        }`}
                      >
                        {c.revoked_at
                          ? 'Revocado'
                          : c.expires_at && c.expires_at < Date.now() / 1000
                            ? 'Caducado'
                            : 'Vigente'}
                      </span>
                    </td>
                    <td className="py-3 px-2 text-right text-gray-400">{timeAgo(c.issued_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}
