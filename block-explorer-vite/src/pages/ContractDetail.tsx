import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getContract, type SmartContract } from '../lib/api'

export default function ContractDetail() {
  const { address } = useParams<{ address: string }>()
  const [contract, setContract] = useState<SmartContract | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!address) return
    getContract(address).then(setContract).catch(() => setError('Contrato no encontrado'))
  }, [address])

  if (error) return <p className="text-red-500">{error}</p>
  if (!contract) return <p className="text-neutral-500">Cargando...</p>

  return (
    <>
      <div className="flex flex-col gap-2 mb-6">
        <div className="flex items-center gap-3">
          <Link to="/contracts" className="text-neutral-500 hover:text-neutral-900 text-sm">&larr; Contratos</Link>
          <h1 className="text-xl font-bold text-neutral-900">Contrato</h1>
        </div>
        <p className="text-sm text-neutral-500 max-w-3xl">
          Estado y codigo almacenados para este contrato en el nodo. La direccion es la identidad del
          contrato en cadena.
        </p>
      </div>

      <div className="bg-white border border-neutral-200 rounded-2xl p-6 mb-6 text-left">
        <div className="grid gap-4 text-sm">
          <div>
            <span className="text-neutral-500">Direccion</span>
            <p className="text-neutral-900 font-mono text-xs break-all mt-1">{contract.address}</p>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <span className="text-neutral-500">Creado</span>
              <p className="text-neutral-900 mt-1">{new Date(contract.created_at * 1000).toLocaleString()}</p>
            </div>
            <div>
              <span className="text-neutral-500">Ultima actualizacion</span>
              <p className="text-neutral-900 mt-1">{new Date(contract.updated_at * 1000).toLocaleString()}</p>
            </div>
            <div>
              <span className="text-neutral-500">Version</span>
              <p className="text-neutral-900 mt-1">{contract.update_sequence}</p>
            </div>
          </div>
        </div>
      </div>

      <h2 className="text-lg font-semibold text-neutral-900 mb-3">Estado</h2>
      <pre className="bg-white border border-neutral-200 rounded-2xl p-4 text-left text-xs font-mono text-neutral-600 overflow-auto mb-6">
        {JSON.stringify(contract.state, null, 2)}
      </pre>

      <h2 className="text-lg font-semibold text-neutral-900 mb-3">Codigo</h2>
      <pre className="bg-white border border-neutral-200 rounded-2xl p-4 text-left text-xs font-mono text-neutral-600 overflow-auto whitespace-pre-wrap">
        {contract.code}
      </pre>
    </>
  )
}
