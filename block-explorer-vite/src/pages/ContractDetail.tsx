import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getContract, type SmartContract } from '../lib/api'

export default function ContractDetail() {
  const { address } = useParams<{ address: string }>()
  const [contract, setContract] = useState<SmartContract | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!address) return
    getContract(address).then(setContract).catch(() => setError('Contract not found'))
  }, [address])

  if (error) return <p className="text-red-400">{error}</p>
  if (!contract) return <p className="text-gray-400">Loading...</p>

  return (
    <>
      <div className="flex flex-col gap-2 mb-6">
        <div className="flex items-center gap-3">
          <Link to="/contracts" className="text-gray-400 hover:text-white text-sm">&larr; Contratos</Link>
          <h1 className="text-xl font-bold text-white">Contrato</h1>
        </div>
        <p className="text-sm text-gray-400 max-w-3xl">
          Estado y código almacenados para este contrato en el nodo. La dirección es la identidad del
          contrato en cadena.
        </p>
      </div>

      <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 mb-6 text-left">
        <div className="grid gap-4 text-sm">
          <div>
            <span className="text-gray-400">Address</span>
            <p className="text-white font-mono text-xs break-all mt-1">{contract.address}</p>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <span className="text-gray-400">Created</span>
              <p className="text-white mt-1">{new Date(contract.created_at * 1000).toLocaleString()}</p>
            </div>
            <div>
              <span className="text-gray-400">Last Updated</span>
              <p className="text-white mt-1">{new Date(contract.updated_at * 1000).toLocaleString()}</p>
            </div>
            <div>
              <span className="text-gray-400">Update Sequence</span>
              <p className="text-white mt-1">{contract.update_sequence}</p>
            </div>
          </div>
        </div>
      </div>

      <h2 className="text-lg font-semibold text-white mb-3">State</h2>
      <pre className="bg-gray-900 border border-gray-800 rounded-xl p-4 text-left text-xs font-mono text-gray-300 overflow-auto mb-6">
        {JSON.stringify(contract.state, null, 2)}
      </pre>

      <h2 className="text-lg font-semibold text-white mb-3">Code</h2>
      <pre className="bg-gray-900 border border-gray-800 rounded-xl p-4 text-left text-xs font-mono text-gray-300 overflow-auto whitespace-pre-wrap">
        {contract.code}
      </pre>
    </>
  )
}
