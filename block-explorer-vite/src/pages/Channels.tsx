import { useEffect, useState } from 'react'
import PageIntro from '../components/PageIntro'
import { listChannels, createChannel, getChannelConfig, type Channel, type ChannelConfig } from '../lib/api'

export default function Channels() {
  const [channels, setChannels] = useState<Channel[]>([])
  const [newId, setNewId] = useState('')
  const [creating, setCreating] = useState(false)
  const [error, setError] = useState('')
  const [selectedConfig, setSelectedConfig] = useState<ChannelConfig | null>(null)

  const load = () => listChannels().then(setChannels).catch(() => {})

  useEffect(() => {
    load()
  }, [])

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newId.trim()) return
    setError('')
    setCreating(true)
    try {
      await createChannel(newId.trim())
      setNewId('')
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error al crear canal')
    } finally {
      setCreating(false)
    }
  }

  const handleViewConfig = async (channelId: string) => {
    try {
      const config = await getChannelConfig(channelId)
      setSelectedConfig(config)
    } catch {
      setSelectedConfig(null)
    }
  }

  return (
    <>
      <PageIntro title="Canales">
        Canales aislados de la red (estilo Hyperledger Fabric). Cada canal tiene su propio ledger y
        world state.
      </PageIntro>

      <div className="bg-white border border-neutral-200 rounded-2xl p-5 mb-8">
        <h2 className="text-lg font-semibold text-neutral-900 mb-4">Crear canal</h2>
        <form onSubmit={handleCreate} className="flex flex-col sm:flex-row gap-4">
          <input
            type="text"
            placeholder="ID del canal (ej. cadena-suministro)"
            value={newId}
            onChange={(e) => setNewId(e.target.value)}
            required
            className="flex-1 border border-neutral-200 rounded-xl px-3 py-2 text-sm
                       focus:outline-none focus:ring-2 focus:ring-main-500"
          />
          <button
            type="submit"
            disabled={creating}
            className="bg-main-500 text-white px-4 py-2 rounded-xl text-sm font-medium
                       hover:bg-main-600 disabled:opacity-50 transition-colors"
          >
            {creating ? 'Creando...' : 'Crear'}
          </button>
        </form>
        {error && <p className="text-red-500 text-sm mt-3">{error}</p>}
      </div>

      <h2 className="text-lg font-semibold text-neutral-900 mb-1">Canales activos</h2>
      <p className="text-xs text-neutral-400 mb-4">Canales registrados en este nodo.</p>

      {channels.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500 mb-2">Sin canales aun.</p>
          <p className="text-neutral-400 text-sm">El canal "default" siempre esta disponible.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-8">
          {channels.map((ch) => (
            <div
              key={ch.channel_id}
              className="bg-white border border-neutral-200 rounded-2xl p-4 flex items-center justify-between"
            >
              <div>
                <p className="text-neutral-900 font-medium">{ch.channel_id}</p>
              </div>
              <button
                onClick={() => handleViewConfig(ch.channel_id)}
                className="text-main-500 hover:text-main-600 text-xs font-medium"
              >
                Configuracion
              </button>
            </div>
          ))}
        </div>
      )}

      {selectedConfig && (
        <div className="bg-white border border-neutral-200 rounded-2xl p-5">
          <h2 className="text-lg font-semibold text-neutral-900 mb-3">
            Configuracion del canal: {selectedConfig.channel_id}
          </h2>
          <dl className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-2 text-sm">
            <dt className="text-neutral-500">Organizaciones miembro</dt>
            <dd className="text-neutral-900">
              {selectedConfig.member_orgs.length > 0
                ? selectedConfig.member_orgs.join(', ')
                : 'Abierto (sin restricciones)'}
            </dd>
            <dt className="text-neutral-500">Peers ancla</dt>
            <dd className="text-neutral-900">
              {selectedConfig.anchor_peers.length > 0
                ? selectedConfig.anchor_peers.join(', ')
                : 'Ninguno'}
            </dd>
            <dt className="text-neutral-500">Tamano max. de bloque</dt>
            <dd className="text-neutral-900">{selectedConfig.max_block_size}</dd>
            <dt className="text-neutral-500">Timeout de lote</dt>
            <dd className="text-neutral-900">{selectedConfig.batch_timeout_ms}ms</dd>
          </dl>
          <button
            onClick={() => setSelectedConfig(null)}
            className="text-neutral-400 hover:text-neutral-600 text-xs mt-4"
          >
            Cerrar
          </button>
        </div>
      )}
    </>
  )
}
