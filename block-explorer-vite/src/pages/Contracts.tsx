import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import PageIntro from '../components/PageIntro'
import { getAllContracts, type SmartContract } from '../lib/api'
import { shortHash } from '../lib/format'

export default function Contracts() {
  const [contracts, setContracts] = useState<SmartContract[]>([])

  useEffect(() => {
    const load = () => getAllContracts().then(setContracts).catch(() => {})
    load()
    const id = setInterval(load, 30000)
    return () => clearInterval(id)
  }, [])

  return (
    <>
      <PageIntro title="Contratos inteligentes">
        Contratos desplegados en este nodo (estado y direccion en cadena). El despliegue suele hacerse
        por API; esta pantalla solo los lista y enlaza al detalle.
      </PageIntro>

      {contracts.length === 0 ? (
        <div className="bg-white border border-neutral-200 rounded-2xl p-8 text-center">
          <p className="text-neutral-500 mb-2">Sin contratos desplegados aun.</p>
          <p className="text-neutral-400 text-sm">Despliega un contrato ERC-20 o NFT via la API.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-neutral-500 text-xs uppercase border-b border-neutral-200">
                <th className="text-left py-3 px-2">Direccion</th>
                <th className="text-right py-3 px-2">Creado</th>
                <th className="text-right py-3 px-2">Actualizado</th>
                <th className="text-right py-3 px-2">Versiones</th>
              </tr>
            </thead>
            <tbody>
              {contracts.map((c) => (
                <tr key={c.address} className="border-b border-neutral-100 hover:bg-white">
                  <td className="py-3 px-2">
                    <Link to={`/contract/${c.address}`} className="text-main-500 hover:text-main-600 font-mono text-xs">
                      {shortHash(c.address)}
                    </Link>
                  </td>
                  <td className="py-3 px-2 text-right text-neutral-500">
                    {new Date(c.created_at * 1000).toLocaleDateString()}
                  </td>
                  <td className="py-3 px-2 text-right text-neutral-500">
                    {new Date(c.updated_at * 1000).toLocaleDateString()}
                  </td>
                  <td className="py-3 px-2 text-right">{c.update_sequence}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
