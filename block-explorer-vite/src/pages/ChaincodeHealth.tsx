import { useState } from 'react';
import PageIntro from '../components/PageIntro';
import { getSandboxReport, type SandboxReport } from '../lib/api';

export default function ChaincodeHealth() {
  const [chaincodeId, setChaincodeId] = useState('');
  const [version, setVersion] = useState('1.0');
  const [report, setReport] = useState<SandboxReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const fetchReport = async () => {
    if (!chaincodeId.trim()) return;
    setLoading(true);
    setError('');
    try {
      const data = await getSandboxReport(chaincodeId.trim(), version.trim());
      setReport(data);
    } catch (e: any) {
      setError(e.message || 'Report not found');
      setReport(null);
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <PageIntro
        title="Chaincode Health"
        description="Reportes de validacion sandbox para contratos desplegados. Verifica que el Wasm cumpla con politicas de seguridad antes de ser aceptado."
      />

      {/* Lookup form */}
      <div className="flex flex-wrap gap-3 mb-6">
        <input
          type="text"
          placeholder="Chaincode ID"
          value={chaincodeId}
          onChange={(e) => setChaincodeId(e.target.value)}
          className="border border-neutral-200 rounded-lg px-3 py-1.5 text-sm w-48"
        />
        <input
          type="text"
          placeholder="Version"
          value={version}
          onChange={(e) => setVersion(e.target.value)}
          className="border border-neutral-200 rounded-lg px-3 py-1.5 text-sm w-24"
        />
        <button
          onClick={fetchReport}
          disabled={loading || !chaincodeId.trim()}
          className="px-4 py-1.5 bg-main-500 text-white rounded-lg text-sm font-medium hover:bg-main-600 disabled:opacity-50"
        >
          {loading ? 'Buscando...' : 'Consultar'}
        </button>
      </div>

      {error && <p className="text-sm text-red-500 mb-4">{error}</p>}

      {report && (
        <div className="space-y-4">
          {/* Summary card */}
          <div className={`border rounded-xl p-5 ${report.passed ? 'border-green-200 bg-green-50' : 'border-red-200 bg-red-50'}`}>
            <div className="flex items-center gap-3 mb-2">
              <span className={`text-xl font-bold ${report.passed ? 'text-green-700' : 'text-red-700'}`}>
                {report.passed ? 'PASSED' : 'FAILED'}
              </span>
              <span className="text-sm text-neutral-500">
                {report.chaincode_id} v{report.version}
              </span>
            </div>
            <div className="flex flex-wrap gap-4 text-xs text-neutral-500">
              <span>Wasm: {(report.wasm_size_bytes / 1024).toFixed(1)} KB</span>
              <span>Validacion: {report.duration_ms}ms</span>
              <span>Checks: {report.checks.length}</span>
            </div>
          </div>

          {/* Check details */}
          <div className="space-y-2">
            {report.checks.map((check, i) => (
              <div
                key={i}
                className={`border rounded-lg px-4 py-3 ${
                  check.passed
                    ? 'border-green-200 bg-white'
                    : 'border-red-200 bg-red-50'
                }`}
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className={`text-sm font-semibold ${check.passed ? 'text-green-700' : 'text-red-700'}`}>
                    {check.passed ? '\u2713' : '\u2717'}
                  </span>
                  <span className="text-sm font-medium text-neutral-700">
                    {check.name.replace(/_/g, ' ')}
                  </span>
                </div>
                <p className="text-xs text-neutral-500 ml-5">{check.detail}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      {!report && !error && !loading && (
        <p className="text-sm text-neutral-400 text-center py-8">
          Ingresa un Chaincode ID y version para consultar su reporte de sandbox.
        </p>
      )}
    </>
  );
}
