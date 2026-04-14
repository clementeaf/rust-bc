import { useState, type ReactElement } from 'react'
import { Link } from 'react-router-dom'
import { createIdentity, getIdentity, type IdentityRecord } from '../lib/api'

const STATUS_OPTIONS: { value: string; label: string }[] = [
  { value: 'active', label: 'Activa (se puede usar)' },
  { value: 'revoked', label: 'Revocada (ya no vale)' },
  { value: 'suspended', label: 'Suspendida (temporalmente en pausa)' },
]

/**
 * Muestra cuánto tiempo pasó desde un instante Unix.
 * @param ts - Segundos desde epoch
 * @returns Texto en español
 */
function timeAgo(ts: number): string {
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 0) return 'en el futuro'
  if (diff < 60) return `hace ${diff}s`
  if (diff < 3600) return `hace ${Math.floor(diff / 60)} min`
  if (diff < 86400) return `hace ${Math.floor(diff / 3600)} h`
  return `hace ${Math.floor(diff / 86400)} d`
}

/**
 * Genera un código de ficha con el formato que espera el nodo.
 * @returns Cadena did:bc:…
 */
function generateDid(): string {
  const hex = Array.from(crypto.getRandomValues(new Uint8Array(8)))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
  return `did:bc:${hex}`
}

export default function Identity(): ReactElement {
  const [didInput, setDidInput] = useState('')
  const [statusInput, setStatusInput] = useState('active')
  const [createMsg, setCreateMsg] = useState('')
  const [createErr, setCreateErr] = useState('')

  const [lookupDid, setLookupDid] = useState('')
  const [identity, setIdentity] = useState<IdentityRecord | null>(null)
  const [lookupErr, setLookupErr] = useState('')

  const handleCreate = async (): Promise<void> => {
    setCreateMsg('')
    setCreateErr('')
    if (!didInput.trim()) {
      setCreateErr('Escribe o genera un código antes de guardar')
      return
    }
    try {
      const rec = await createIdentity(didInput.trim(), statusInput || 'active')
      setCreateMsg('Listo. Este código quedó guardado en el nodo.')
      setIdentity(rec)
    } catch (e: unknown) {
      setCreateErr(e instanceof Error ? e.message : 'No se pudo guardar')
    }
  }

  const handleLookup = async (): Promise<void> => {
    setLookupErr('')
    setIdentity(null)
    if (!lookupDid.trim()) {
      setLookupErr('Pega aquí el código completo que quieres buscar')
      return
    }
    try {
      const rec = await getIdentity(lookupDid.trim())
      setIdentity(rec)
    } catch (e: unknown) {
      setLookupErr(e instanceof Error ? e.message : 'No encontramos ese código en el nodo')
    }
  }

  return (
    <div className="space-y-8">
      <header className="max-w-3xl">
        <h1 className="text-2xl font-bold text-white tracking-tight">Personas y organizaciones</h1>
        <p className="text-gray-300 text-base mt-3 leading-relaxed">
          Cada persona, empresa u organismo puede tener un <strong className="text-white">código único</strong>{' '}
          en este sistema. Sirve para nombrarla en listas y registros.{' '}
          <span className="text-gray-400">
            No es una contraseña ni un documento oficial: es solo un identificador guardado en este servidor.
          </span>
        </p>
        <p className="text-sm text-gray-500 mt-4">
          ¿Necesitas que una entidad &quot;certifique&quot; algo sobre otra? Eso se hace en{' '}
          <Link to="/credentials" className="text-cyan-400 hover:text-cyan-300 underline underline-offset-2">
            Certificados entre fichas
          </Link>
          , después de crear aquí los dos códigos.
        </p>
      </header>

      <div className="rounded-xl border border-gray-700/80 bg-gray-900/50 p-5 text-sm text-gray-300 max-w-3xl">
        <p className="font-medium text-white mb-2">En una frase</p>
        <p className="leading-relaxed">
          <strong className="text-gray-200">Paso 1:</strong> das de alta un código.{' '}
          <strong className="text-gray-200">Paso 2:</strong> si hace falta, lo buscas para ver si el nodo lo tiene
          guardado.
        </p>
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-white">Dar de alta un código nuevo</h2>
        <p className="text-sm text-gray-400 mt-2 mb-6 leading-relaxed">
          Pulsa &quot;Crear código automático&quot; o escribe uno tuyo. El formato técnico empieza por{' '}
          <code className="text-cyan-400/90 text-xs">did:bc:</code> seguido de letras y números.
        </p>
        <div className="space-y-5">
          <div>
            <label htmlFor="did-create" className="block text-sm font-medium text-gray-200 mb-1.5">
              Código de la ficha
            </label>
            <p className="text-xs text-gray-500 mb-2">Identifica a esta persona u organización en este nodo.</p>
            <div className="flex flex-col sm:flex-row gap-2">
              <input
                id="did-create"
                value={didInput}
                onChange={(e) => setDidInput(e.target.value)}
                placeholder="did:bc:…"
                className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
              />
              <button
                type="button"
                onClick={() => setDidInput(generateDid())}
                className="px-4 py-2.5 text-sm bg-gray-800 text-gray-200 rounded-lg border border-gray-700 hover:bg-gray-700 whitespace-nowrap"
              >
                Crear código automático
              </button>
            </div>
          </div>
          <div>
            <label htmlFor="status-create" className="block text-sm font-medium text-gray-200 mb-1.5">
              Situación de esta ficha
            </label>
            <p className="text-xs text-gray-500 mb-2">Suele dejarse en &quot;Activa&quot; salvo que quieras bloquearla.</p>
            <select
              id="status-create"
              value={statusInput}
              onChange={(e) => setStatusInput(e.target.value)}
              className="w-full sm:max-w-md bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm focus:outline-none focus:border-cyan-500"
            >
              {STATUS_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
          </div>
          <button
            type="button"
            onClick={() => void handleCreate()}
            className="px-5 py-2.5 bg-cyan-600 hover:bg-cyan-500 text-white text-sm font-medium rounded-lg transition-colors"
          >
            Guardar en el nodo
          </button>
        </div>
        {createMsg && <p className="mt-4 text-sm text-green-400">{createMsg}</p>}
        {createErr && <p className="mt-4 text-sm text-red-400">{createErr}</p>}
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-white">Buscar un código que ya exista</h2>
        <p className="text-sm text-gray-400 mt-2 mb-5 leading-relaxed">
          Solo consulta: comprueba si el nodo tiene guardada esa ficha y en qué estado está.
        </p>
        <div className="flex flex-col sm:flex-row gap-3">
          <div className="flex-1">
            <label htmlFor="did-lookup" className="block text-sm font-medium text-gray-200 mb-1.5">
              Código a buscar
            </label>
            <input
              id="did-lookup"
              value={lookupDid}
              onChange={(e) => setLookupDid(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void handleLookup()}
              placeholder="Pega el código completo"
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-white text-sm font-mono placeholder-gray-500 focus:outline-none focus:border-cyan-500"
            />
          </div>
          <div className="sm:self-end">
            <button
              type="button"
              onClick={() => void handleLookup()}
              className="w-full sm:w-auto px-5 py-2.5 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium rounded-lg transition-colors"
            >
              Buscar
            </button>
          </div>
        </div>
        {lookupErr && <p className="mt-4 text-sm text-red-400">{lookupErr}</p>}
      </div>

      {identity && (
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 max-w-3xl">
          <h2 className="text-lg font-semibold text-white mb-4">Resultado</h2>
          <dl className="space-y-3 text-sm">
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-40 shrink-0">Código</dt>
              <dd className="text-cyan-400 font-mono break-all">{identity.did}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-40 shrink-0">Situación</dt>
              <dd>
                <span
                  className={`inline-flex px-2 py-0.5 rounded text-xs font-medium ${
                    identity.status === 'active'
                      ? 'bg-green-900/50 text-green-300'
                      : identity.status === 'revoked'
                        ? 'bg-red-900/50 text-red-300'
                        : 'bg-gray-800 text-gray-300'
                  }`}
                >
                  {identity.status === 'active'
                    ? 'Activa'
                    : identity.status === 'revoked'
                      ? 'Revocada'
                      : identity.status === 'suspended'
                        ? 'Suspendida'
                        : identity.status}
                </span>
              </dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-40 shrink-0">Registrada</dt>
              <dd className="text-gray-200">{timeAgo(identity.created_at)}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-gray-400 w-40 shrink-0">Última actualización</dt>
              <dd className="text-gray-200">{timeAgo(identity.updated_at)}</dd>
            </div>
          </dl>
        </div>
      )}
    </div>
  )
}
