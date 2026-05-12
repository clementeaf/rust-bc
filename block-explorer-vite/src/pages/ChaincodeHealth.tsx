import { useEffect, useState, useCallback } from 'react';
import {
  getHealth,
  getStressReport,
  getRegulatoryChecks,
  getVersion,
  type HealthResponse,
  type StressReport,
  type RegulatoryChecks,
  type VersionResponse,
} from '../lib/api';

export default function ChaincodeHealth() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [version, setVersion] = useState<VersionResponse | null>(null);
  const [stress, setStress] = useState<StressReport | null>(null);
  const [regulatory, setRegulatory] = useState<RegulatoryChecks | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastRefresh, setLastRefresh] = useState(new Date());

  const fetchAll = useCallback(async () => {
    try {
      const [h, v, s, r] = await Promise.all([
        getHealth().catch(() => null),
        getVersion().catch(() => null),
        getStressReport(200).catch(() => null),
        getRegulatoryChecks().catch(() => null),
      ]);
      setHealth(h);
      setVersion(v);
      setStress(s);
      setRegulatory(r);
      setLastRefresh(new Date());
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, 60_000);
    return () => clearInterval(interval);
  }, [fetchAll]);

  return (
    <div className="flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-bold text-neutral-900 tracking-tight">Salud del Sistema</h1>
          <p className="text-xs text-neutral-400 mt-0.5">
            Estado de componentes, rendimiento y cumplimiento regulatorio · Refresh cada 60s
          </p>
        </div>
        <p className="text-[10px] text-neutral-400">{lastRefresh.toLocaleTimeString('es-CL')}</p>
      </div>

      {loading ? (
        <p className="text-sm text-neutral-400 py-8 text-center">Evaluando salud del sistema...</p>
      ) : (
        <>
          {/* Overview cards */}
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
            <OverviewCard
              label="Estado general"
              value={health?.status === 'healthy' ? 'Saludable' : health?.status || '?'}
              ok={health?.status === 'healthy'}
            />
            <OverviewCard
              label="Version"
              value={version ? `v${version.rust_bc_version}` : '?'}
              sub={version ? `API ${version.api_version}` : ''}
              ok
            />
            <OverviewCard
              label="Uptime"
              value={health ? formatUptime(health.uptime_seconds) : '?'}
              ok={!!health}
            />
            <OverviewCard
              label="Altura blockchain"
              value={String(health?.blockchain.height ?? '?')}
              sub={`${health?.blockchain.validators_count ?? 0} validadores`}
              ok={(health?.blockchain.height ?? 0) > 0}
            />
          </div>

          {/* Infrastructure checks */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-4">
            {/* Infra */}
            <div className="bg-white border border-neutral-200 rounded-xl px-5 py-4">
              <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-3">Infraestructura</p>
              <div className="space-y-2">
                <CheckRow label="Almacenamiento" value={health?.checks?.storage || '?'} ok={health?.checks?.storage === 'ok'} />
                <CheckRow label="Peers de red" value={health?.checks?.peers || '?'} ok={health?.checks?.peers !== 'none'} />
                <CheckRow label="Servicio de orden" value={health?.checks?.ordering || '?'} ok={health?.checks?.ordering === 'ok'} />
              </div>
            </div>

            {/* Regulatory */}
            <div className="bg-white border border-neutral-200 rounded-xl px-5 py-4">
              <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold mb-3">Cumplimiento regulatorio</p>
              {regulatory ? (
                <>
                  <div className="flex items-center gap-3 mb-3">
                    <span className={`text-2xl font-bold ${regulatory.summary.failed === 0 ? 'text-emerald-600' : 'text-amber-600'}`}>
                      {regulatory.summary.pass_rate_pct.toFixed(0)}%
                    </span>
                    <span className="text-xs text-neutral-500">
                      {regulatory.summary.passed}/{regulatory.summary.total} checks aprobados
                    </span>
                  </div>
                  <div className="flex h-2 rounded-full overflow-hidden bg-neutral-100 mb-3">
                    <div className="bg-emerald-500" style={{ width: `${regulatory.summary.pass_rate_pct}%` }} />
                    {regulatory.summary.failed > 0 && (
                      <div className="bg-red-500" style={{ width: `${(regulatory.summary.failed / regulatory.summary.total) * 100}%` }} />
                    )}
                  </div>
                  <div className="space-y-1 max-h-40 overflow-y-auto">
                    {regulatory.checks.filter((c) => c.status === 'Fail').map((c) => (
                      <div key={c.id} className="flex items-center gap-2 text-xs">
                        <span className="w-1.5 h-1.5 rounded-full bg-red-500 flex-shrink-0" />
                        <span className="text-red-700">{c.description}</span>
                      </div>
                    ))}
                    {regulatory.summary.failed === 0 && (
                      <p className="text-xs text-emerald-600">Todos los checks regulatorios aprobados.</p>
                    )}
                  </div>
                </>
              ) : (
                <p className="text-xs text-neutral-400">No disponible</p>
              )}
            </div>
          </div>

          {/* Stress test results */}
          {stress && (
            <div className="bg-white border border-neutral-200 rounded-xl px-5 py-4">
              <div className="flex items-center justify-between mb-3">
                <p className="text-[10px] text-neutral-400 uppercase tracking-wider font-bold">Rendimiento por modulo</p>
                <span className="text-[10px] text-neutral-400">
                  {stress.passed}/{stress.total_modules} OK · {stress.degraded} degradados · {stress.failed} fallidos
                </span>
              </div>
              <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-2">
                {stress.results.map((m) => (
                  <div key={m.module} className={`rounded-lg px-3 py-2.5 border ${
                    m.status === 'Pass' ? 'bg-emerald-50 border-emerald-200'
                      : m.status === 'Degraded' ? 'bg-amber-50 border-amber-200'
                      : 'bg-red-50 border-red-200'
                  }`}>
                    <div className="flex items-center justify-between mb-1">
                      <p className="text-[10px] font-semibold text-neutral-700 truncate">{m.module.replace(/_/g, ' ')}</p>
                      <span className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                        m.status === 'Pass' ? 'bg-emerald-500' : m.status === 'Degraded' ? 'bg-amber-500' : 'bg-red-500'
                      }`} />
                    </div>
                    <p className="text-sm font-bold text-neutral-800">
                      {m.ops_per_sec.toFixed(0)}
                      <span className="text-[9px] font-normal text-neutral-400"> ops/s</span>
                    </p>
                    <p className="text-[9px] text-neutral-400">p99 {m.p99_us}us · {m.errors} err</p>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function OverviewCard({ label, value, sub, ok }: { label: string; value: string; sub?: string; ok: boolean }) {
  return (
    <div className={`rounded-xl border px-4 py-3 ${ok ? 'bg-emerald-50 border-emerald-200' : 'bg-amber-50 border-amber-200'}`}>
      <p className="text-[9px] text-neutral-400 uppercase">{label}</p>
      <p className={`text-lg font-bold ${ok ? 'text-emerald-700' : 'text-amber-700'}`}>{value}</p>
      {sub && <p className="text-[10px] text-neutral-500">{sub}</p>}
    </div>
  );
}

function CheckRow({ label, value, ok }: { label: string; value: string; ok: boolean }) {
  return (
    <div className="flex items-center justify-between py-1.5 border-b border-neutral-100 last:border-0">
      <span className="text-xs text-neutral-700">{label}</span>
      <div className="flex items-center gap-2">
        <span className="text-xs text-neutral-500">{value}</span>
        <span className={`w-2 h-2 rounded-full ${ok ? 'bg-emerald-500' : 'bg-amber-500'}`} />
      </div>
    </div>
  );
}
