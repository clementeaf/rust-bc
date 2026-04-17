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
        <h1 className="text-2xl font-bold text-neutral-900 tracking-tight">Personas y organizaciones</h1>
        <p className="text-neutral-600 text-base mt-3 leading-relaxed">
          Cada persona, empresa u organismo puede tener un <strong className="text-neutral-900">código único</strong>{' '}
          en este sistema. Sirve para nombrarla en listas y registros.{' '}
          <span className="text-neutral-500">
            No es una contraseña ni un documento oficial: es solo un identificador guardado en este servidor.
          </span>
        </p>
        <p className="text-sm text-neutral-400 mt-4">
          ¿Necesitas que una entidad &quot;certifique&quot; algo sobre otra? Eso se hace en{' '}
          <Link to="/credentials" className="text-main-500 hover:text-main-600 underline underline-offset-2">
            Certificados entre fichas
          </Link>
          , después de crear aquí los dos códigos.
        </p>
      </header>

      <div className="rounded-2xl border border-neutral-200 bg-white p-5 text-sm text-neutral-600 max-w-3xl">
        <p className="font-medium text-neutral-900 mb-2">En una frase</p>
        <p className="leading-relaxed">
          <strong className="text-neutral-700">Paso 1:</strong> das de alta un código.{' '}
          <strong className="text-neutral-700">Paso 2:</strong> si hace falta, lo buscas para ver si el nodo lo tiene
          guardado.
        </p>
      </div>

      <div className="bg-white border border-neutral-200 rounded-2xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-neutral-900">Dar de alta un código nuevo</h2>
        <p className="text-sm text-neutral-500 mt-2 mb-6 leading-relaxed">
          Pulsa &quot;Crear código automático&quot; o escribe uno tuyo. El formato técnico empieza por{' '}
          <code className="text-main-500 text-xs">did:bc:</code> seguido de letras y números.
        </p>
        <div className="space-y-5">
          <div>
            <label htmlFor="did-create" className="block text-sm font-medium text-neutral-700 mb-1.5">
              Código de la ficha
            </label>
            <p className="text-xs text-neutral-400 mb-2">Identifica a esta persona u organización en este nodo.</p>
            <div className="flex flex-col sm:flex-row gap-2">
              <input
                id="did-create"
                value={didInput}
                onChange={(e) => setDidInput(e.target.value)}
                placeholder="did:bc:…"
                className="flex-1 bg-neutral-100 border border-neutral-200 rounded-lg px-3 py-2.5 text-neutral-900 text-sm font-mono placeholder-neutral-400 focus:outline-none focus:border-main-500"
              />
              <button
                type="button"
                onClick={() => setDidInput(generateDid())}
                className="px-4 py-2.5 text-sm bg-neutral-100 text-neutral-700 rounded-lg border border-neutral-200 hover:bg-neutral-200 whitespace-nowrap"
              >
                Crear código automático
              </button>
            </div>
          </div>
          <div>
            <label htmlFor="status-create" className="block text-sm font-medium text-neutral-700 mb-1.5">
              Situación de esta ficha
            </label>
            <p className="text-xs text-neutral-400 mb-2">Suele dejarse en &quot;Activa&quot; salvo que quieras bloquearla.</p>
            <select
              id="status-create"
              value={statusInput}
              onChange={(e) => setStatusInput(e.target.value)}
              className="w-full sm:max-w-md bg-neutral-100 border border-neutral-200 rounded-lg px-3 py-2.5 text-neutral-900 text-sm focus:outline-none focus:border-main-500"
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
            className="px-5 py-2.5 bg-main-500 hover:bg-main-600 text-neutral-900 text-sm font-medium rounded-lg transition-colors"
          >
            Guardar en el nodo
          </button>
        </div>
        {createMsg && <p className="mt-4 text-sm text-green-600">{createMsg}</p>}
        {createErr && <p className="mt-4 text-sm text-red-500">{createErr}</p>}
      </div>

      <div className="bg-white border border-neutral-200 rounded-2xl p-6 max-w-3xl">
        <h2 className="text-lg font-semibold text-neutral-900">Buscar un código que ya exista</h2>
        <p className="text-sm text-neutral-500 mt-2 mb-5 leading-relaxed">
          Solo consulta: comprueba si el nodo tiene guardada esa ficha y en qué estado está.
        </p>
        <div className="flex flex-col sm:flex-row gap-3">
          <div className="flex-1">
            <label htmlFor="did-lookup" className="block text-sm font-medium text-neutral-700 mb-1.5">
              Código a buscar
            </label>
            <input
              id="did-lookup"
              value={lookupDid}
              onChange={(e) => setLookupDid(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && void handleLookup()}
              placeholder="Pega el código completo"
              className="w-full bg-neutral-100 border border-neutral-200 rounded-lg px-3 py-2.5 text-neutral-900 text-sm font-mono placeholder-neutral-400 focus:outline-none focus:border-main-500"
            />
          </div>
          <div className="sm:self-end">
            <button
              type="button"
              onClick={() => void handleLookup()}
              className="w-full sm:w-auto px-5 py-2.5 bg-gray-700 hover:bg-neutral-200 text-neutral-900 text-sm font-medium rounded-lg transition-colors"
            >
              Buscar
            </button>
          </div>
        </div>
        {lookupErr && <p className="mt-4 text-sm text-red-500">{lookupErr}</p>}
      </div>

      {identity && (
        <div className="bg-white border border-neutral-200 rounded-2xl p-6 max-w-3xl">
          <h2 className="text-lg font-semibold text-neutral-900 mb-4">Resultado</h2>
          <dl className="space-y-3 text-sm">
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-neutral-500 w-40 shrink-0">Código</dt>
              <dd className="text-main-500 font-mono break-all">{identity.did}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-neutral-500 w-40 shrink-0">Situación</dt>
              <dd>
                <span
                  className={`inline-flex px-2 py-0.5 rounded text-xs font-medium ${
                    identity.status === 'active'
                      ? 'bg-green-50 text-green-600'
                      : identity.status === 'revoked'
                        ? 'bg-red-50 text-red-500'
                        : 'bg-neutral-100 text-neutral-600'
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
              <dt className="text-neutral-500 w-40 shrink-0">Registrada</dt>
              <dd className="text-neutral-700">{timeAgo(identity.created_at)}</dd>
            </div>
            <div className="flex flex-col sm:flex-row sm:gap-4">
              <dt className="text-neutral-500 w-40 shrink-0">Última actualización</dt>
              <dd className="text-neutral-700">{timeAgo(identity.updated_at)}</dd>
            </div>
          </dl>
        </div>
      )}
    </div>
  )
}
