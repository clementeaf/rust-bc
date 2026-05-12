import { useEffect, useState, useCallback } from 'react';
import { Link } from 'react-router-dom';
import {
  getHealth,
  getStats,
  getRegulatoryChecks,
  getPentestReport,
  getStressReport,
  getForensicIntegrity,
  getForensicSecurity,
  getOracleStatus,
  getValidators,
  type HealthResponse,
  type Stats,
  type RegulatoryChecks,
  type PentestReport,
  type StressReport,
  type IntegrityResult,
  type ForensicSecurityResponse,
  type OracleStatus,
  type Validator,
} from '../lib/api';

// ── Horizontal service state ────────────────────────────────────────────────

interface ServiceStatus {
  name: string;
  label: string;
  status: 'ok' | 'degraded' | 'error' | 'loading';
  detail: string;
  iso: string;
}

function deriveServices(
  health: HealthResponse | null,
  stats: Stats | null,
  regulatory: RegulatoryChecks | null,
  pentest: PentestReport | null,
  stress: StressReport | null,
  integrity: IntegrityResult | null,
  oracle: OracleStatus | null,
  validators: Validator[],
): ServiceStatus[] {
  return [
    {
      name: 'security',
      label: 'Seguridad',
      status: pentest ? (pentest.vulnerable === 0 ? 'ok' : 'error') : 'loading',
      detail: pentest
        ? `${pentest.blocked}/${pentest.total_scenarios} bloqueados`
        : 'Ejecutando...',
      iso: 'ISO 27001',
    },
    {
      name: 'forensic',
      label: 'Forense',
      status: integrity ? (integrity.status === 'Valid' ? 'ok' : 'error') : 'loading',
      detail: integrity
        ? integrity.status === 'Valid'
          ? `${integrity.blocks_checked} bloques OK`
          : `${integrity.mismatches.length} errores`
        : 'Verificando...',
      iso: 'ISO 27037',
    },
    {
      name: 'compliance',
      label: 'Regulatorio',
      status: regulatory ? (regulatory.summary.failed === 0 ? 'ok' : 'degraded') : 'loading',
      detail: regulatory
        ? `${regulatory.summary.passed}/${regulatory.summary.total} (${regulatory.summary.pass_rate_pct.toFixed(0)}%)`
        : 'Evaluando...',
      iso: 'Ley 21.663',
    },
    {
      name: 'crypto',
      label: 'Criptografia',
      status: stress
        ? stress.results.find((r) => r.module === 'crypto_hash')?.status === 'Pass' ? 'ok' : 'degraded'
        : 'loading',
      detail: stress
        ? (() => {
            const c = stress.results.find((r) => r.module === 'crypto_hash');
            return c ? `${c.ops_per_sec.toFixed(0)} ops/s` : '—';
          })()
        : 'Testeando...',
      iso: 'FIPS 140-3',
    },
    {
      name: 'storage',
      label: 'Storage',
      status: health ? (health.checks?.storage === 'ok' ? 'ok' : 'degraded') : 'loading',
      detail: health
        ? `${health.checks?.storage || '?'} — ${stats ? stats.blockchain.block_count + ' bloques' : ''}`
        : 'Consultando...',
      iso: 'ISO 27001',
    },
    {
      name: 'consensus',
      label: 'Consenso',
      status: stats ? (stats.blockchain.block_count > 0 ? 'ok' : 'degraded') : 'loading',
      detail: stats
        ? `Altura ${stats.blockchain.latest_block_index}, ${validators.length} val.`
        : 'Consultando...',
      iso: 'ISO 22739',
    },
    {
      name: 'oracles',
      label: 'Oraculos',
      status: oracle ? (oracle.stale_feeds === 0 ? 'ok' : 'degraded') : 'loading',
      detail: oracle
        ? `${oracle.fresh_feeds} frescos, ${oracle.stale_feeds} stale`
        : 'Consultando...',
      iso: 'ISO 20022',
    },
    {
      name: 'intelligence',
      label: 'AML/IA',
      status: stress
        ? (() => {
            const aml = stress.results.find((r) => r.module === 'risk_scoring' || r.module === 'anomaly_detection');
            return aml?.status === 'Pass' ? 'ok' : 'degraded';
          })()
        : 'loading',
      detail: stress
        ? (() => {
            const risk = stress.results.find((r) => r.module === 'risk_scoring');
            return risk ? `${risk.ops_per_sec.toFixed(0)} ops/s` : '—';
          })()
        : 'Evaluando...',
      iso: 'AML/CFT',
    },
  ];
}

const STATUS_STYLES: Record<string, { bg: string; text: string; dot: string; label: string }> = {
  ok:       { bg: 'bg-emerald-50',  text: 'text-emerald-700', dot: 'bg-emerald-500', label: 'OK' },
  degraded: { bg: 'bg-amber-50',    text: 'text-amber-700',   dot: 'bg-amber-500',   label: 'Degradado' },
  error:    { bg: 'bg-red-50',      text: 'text-red-700',     dot: 'bg-red-500',     label: 'Critico' },
  loading:  { bg: 'bg-neutral-50',  text: 'text-neutral-400', dot: 'bg-neutral-300', label: '...' },
};

// ── Verticals ───────────────────────────────────────────────────────────────

interface VerticalCard {
  label: string;
  href: string;
  stat: string | number;
  statLabel: string;
}

function buildVerticals(stats: Stats | null): VerticalCard[] {
  return [
    { label: 'Credenciales', href: '/identity', stat: '-', statLabel: 'DIDs' },
    { label: 'Votacion', href: '/governance', stat: '-', statLabel: 'Propuestas' },
    { label: 'Finanzas', href: '/wallets', stat: stats?.blockchain.unique_addresses ?? '-', statLabel: 'Wallets' },
    { label: 'Contratos', href: '/contracts', stat: '-', statLabel: 'Desplegados' },
  ];
}

// ── Component ───────────────────────────────────────────────────────────────

export default function Integridad() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [stats, setStats] = useState<Stats | null>(null);
  const [regulatory, setRegulatory] = useState<RegulatoryChecks | null>(null);
  const [pentest, setPentest] = useState<PentestReport | null>(null);
  const [stress, setStress] = useState<StressReport | null>(null);
  const [integrity, setIntegrity] = useState<IntegrityResult | null>(null);
  const [forensicSec, setForensicSec] = useState<ForensicSecurityResponse | null>(null);
  const [oracle, setOracle] = useState<OracleStatus | null>(null);
  const [validators, setValidators] = useState<Validator[]>([]);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const [error, setError] = useState('');
  const [activeTab, setActiveTab] = useState<'report' | 'events'>('report');
  const [drawerService, setDrawerService] = useState<string | null>(null);

  const fetchAll = useCallback(async () => {
    try {
      const [h, s, v, o] = await Promise.all([
        getHealth().catch(() => null),
        getStats().catch(() => null),
        getValidators().catch(() => []),
        getOracleStatus().catch(() => null),
      ]);
      setHealth(h);
      setStats(s);
      setValidators(v);
      setOracle(o);
      setError('');
      setLastRefresh(new Date());
    } catch (e: any) {
      setError(e.message || 'Error de conexion');
    }
  }, []);

  const fetchReports = useCallback(async () => {
    const [r, p, st, fi, fs] = await Promise.all([
      getRegulatoryChecks().catch(() => null),
      getPentestReport().catch(() => null),
      getStressReport(500).catch(() => null),
      getForensicIntegrity().catch(() => null),
      getForensicSecurity({ limit: 50 }).catch(() => null),
    ]);
    setRegulatory(r);
    setPentest(p);
    setStress(st);
    setIntegrity(fi);
    setForensicSec(fs);
  }, []);

  useEffect(() => {
    fetchAll();
    fetchReports();
    const lightInterval = setInterval(fetchAll, 30_000);
    const heavyInterval = setInterval(fetchReports, 300_000);
    return () => {
      clearInterval(lightInterval);
      clearInterval(heavyInterval);
    };
  }, [fetchAll, fetchReports]);

  const services = deriveServices(health, stats, regulatory, pentest, stress, integrity, oracle, validators);
  const verticals = buildVerticals(stats);

  const overallOk = services.every((s) => s.status === 'ok' || s.status === 'loading');
  const overallLoading = services.some((s) => s.status === 'loading');

  return (
    <div className="flex flex-col">
      {/* ── Header row: title + status ── */}
      <div className="flex items-center justify-between mb-3 flex-shrink-0">
        <div>
          <h1 className="text-xl font-bold text-neutral-900 tracking-tight">Integridad de la Plataforma</h1>
          <p className="text-xs text-neutral-400 mt-0.5">
            Ultima verificacion: {lastRefresh.toLocaleTimeString('es-CL')} · Auto-refresh 30s
            {health ? ` · Uptime ${formatUptime(health.uptime_seconds)}` : ''}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span
            className={`w-2.5 h-2.5 rounded-full ${
              overallLoading ? 'bg-neutral-300 animate-pulse' : overallOk ? 'bg-emerald-500' : 'bg-amber-500'
            }`}
          />
          <span className="text-sm font-medium text-neutral-700">
            {overallLoading ? 'Evaluando...' : overallOk ? 'Operativo' : 'Atencion requerida'}
          </span>
        </div>
      </div>

      {error && <p className="text-xs text-red-500 mb-2 flex-shrink-0">{error}</p>}

      {/* ── Main 2-column layout ── */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Left column: services + verticals */}
        <div className="lg:col-span-1 flex flex-col gap-3">
          {/* Services */}
          <div className="grid grid-cols-2 gap-2">
            {services.map((svc) => {
              const style = STATUS_STYLES[svc.status];
              return (
                <button
                  key={svc.name}
                  onClick={() => setDrawerService(svc.name)}
                  className={`rounded-lg border px-3 py-2.5 text-left transition-all hover:shadow-sm hover:scale-[1.02] cursor-pointer ${style.bg} border-neutral-200`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs font-semibold text-neutral-800">{svc.label}</span>
                    <span className="flex items-center gap-1">
                      <span className={`w-1.5 h-1.5 rounded-full ${style.dot}`} />
                      <span className={`text-[10px] font-medium ${style.text}`}>{style.label}</span>
                    </span>
                  </div>
                  <p className="text-[11px] font-mono text-neutral-600 leading-tight">{svc.detail}</p>
                  <p className="text-[9px] text-neutral-400 mt-1">{svc.iso}</p>
                </button>
              );
            })}
          </div>

          {/* Verticals */}
          <div>
            <p className="text-[10px] font-bold text-neutral-400 uppercase tracking-widest mb-2">Verticales</p>
            <div className="grid grid-cols-2 gap-2">
              {verticals.map((v) => (
                <Link
                  key={v.label}
                  to={v.href}
                  className="bg-white border border-neutral-200 rounded-lg px-3 py-2.5 hover:border-main-300 transition-colors"
                >
                  <p className="text-xs font-semibold text-neutral-800">{v.label}</p>
                  <div className="mt-1">
                    <p className="text-[9px] text-neutral-400 uppercase">{v.statLabel}</p>
                    <p className="text-base font-bold text-neutral-800 leading-tight">{v.stat}</p>
                  </div>
                </Link>
              ))}
            </div>
          </div>
        </div>

        {/* Right column: report / events (tabbed) */}
        <div className="lg:col-span-2 flex flex-col">
          {/* Tabs */}
          <div className="flex items-center gap-1 mb-2 flex-shrink-0">
            <button
              onClick={() => setActiveTab('report')}
              className={`text-xs px-3 py-1.5 rounded-lg font-medium transition-colors ${
                activeTab === 'report'
                  ? 'bg-main-500 text-white'
                  : 'text-neutral-500 hover:bg-neutral-100'
              }`}
            >
              Reporte de integridad
            </button>
            <button
              onClick={() => setActiveTab('events')}
              className={`text-xs px-3 py-1.5 rounded-lg font-medium transition-colors ${
                activeTab === 'events'
                  ? 'bg-main-500 text-white'
                  : 'text-neutral-500 hover:bg-neutral-100'
              }`}
            >
              Eventos de seguridad
              {forensicSec && forensicSec.summary.critical > 0 && (
                <span className="ml-1.5 bg-red-500 text-white text-[10px] px-1.5 py-0.5 rounded-full">
                  {forensicSec.summary.critical}
                </span>
              )}
            </button>
            <div className="flex-1" />
            <button
              onClick={() => window.print()}
              className="text-[10px] border border-neutral-200 rounded-lg px-2.5 py-1 text-neutral-500 hover:bg-neutral-50 print:hidden"
            >
              Exportar PDF
            </button>
          </div>

          {/* Tab content */}
          <div className="rounded-xl border border-neutral-200 bg-white print:break-inside-avoid">
            {activeTab === 'report' ? (
              <div>
                <div className="px-4 py-2.5 border-b border-neutral-100 flex items-center justify-between">
                  <div>
                    <p className="font-semibold text-neutral-800 text-sm">Cerulean Ledger — Reporte de integridad</p>
                    <p className="text-[10px] text-neutral-400">
                      {new Date().toLocaleDateString('es-CL', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' })}
                    </p>
                  </div>
                </div>
                <table className="w-full text-sm">
                  <thead>
                    <tr className="text-left text-[10px] text-neutral-400 border-b border-neutral-200 uppercase tracking-wider">
                      <th className="px-4 py-1.5">Servicio</th>
                      <th className="px-4 py-1.5">Normativa</th>
                      <th className="px-4 py-1.5">Estado</th>
                      <th className="px-4 py-1.5">Detalle</th>
                    </tr>
                  </thead>
                  <tbody>
                    {services.map((svc) => {
                      const style = STATUS_STYLES[svc.status];
                      return (
                        <tr key={svc.name} className="border-b border-neutral-100">
                          <td className="px-4 py-1.5 font-medium text-neutral-800 text-xs">{svc.label}</td>
                          <td className="px-4 py-1.5 text-[10px] text-neutral-500">{svc.iso}</td>
                          <td className="px-4 py-1.5">
                            <span className={`inline-flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-full ${style.bg} ${style.text}`}>
                              <span className={`w-1.5 h-1.5 rounded-full ${style.dot}`} />
                              {style.label}
                            </span>
                          </td>
                          <td className="px-4 py-1.5 text-[11px] font-mono text-neutral-600">{svc.detail}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
                {regulatory && (
                  <div className="px-4 py-2 border-t border-neutral-100 text-[10px] text-neutral-500">
                    Regulatorio: {regulatory.summary.passed}/{regulatory.summary.total} ({regulatory.summary.pass_rate_pct.toFixed(1)}%)
                    {pentest && ` · Pentest: ${pentest.blocked + pentest.detected}/${pentest.total_scenarios} mitigados`}
                    {stress && ` · Stress: ${stress.passed}/${stress.total_modules} OK`}
                  </div>
                )}
                {stress && (
                  <div className="border-t border-neutral-100 px-4 py-3">
                    <p className="text-[10px] font-bold text-neutral-400 uppercase tracking-widest mb-2">Rendimiento por modulo</p>
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                      {stress.results.map((m) => (
                        <div key={m.module} className="rounded-lg bg-neutral-50 px-3 py-2">
                          <p className="text-[10px] font-semibold text-neutral-700 truncate">{m.module.replace(/_/g, ' ')}</p>
                          <p className="text-sm font-bold text-neutral-800">{m.ops_per_sec.toFixed(0)}<span className="text-[9px] font-normal text-neutral-400"> ops/s</span></p>
                          <div className="flex items-center gap-2 mt-0.5">
                            <span className="text-[9px] text-neutral-400">p99 {m.p99_us}us</span>
                            <span className={`w-1.5 h-1.5 rounded-full ${m.status === 'Pass' ? 'bg-emerald-500' : m.status === 'Degraded' ? 'bg-amber-500' : 'bg-red-500'}`} />
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <>
                {forensicSec && forensicSec.events.length > 0 ? (
                  <>
                    <div className="grid grid-cols-4 gap-2 p-3 border-b border-neutral-100">
                      <MiniSeverity label="Criticos" value={forensicSec.summary.critical} color="red" />
                      <MiniSeverity label="Altos" value={forensicSec.summary.high} color="amber" />
                      <MiniSeverity label="Medios" value={forensicSec.summary.medium} color="yellow" />
                      <MiniSeverity label="Bajos" value={forensicSec.summary.low} color="neutral" />
                    </div>
                    <table className="w-full text-sm">
                      <thead>
                        <tr className="text-left text-[10px] text-neutral-400 border-b border-neutral-200 uppercase tracking-wider">
                          <th className="px-4 py-1.5">Timestamp</th>
                          <th className="px-4 py-1.5">Tipo</th>
                          <th className="px-4 py-1.5">Severidad</th>
                          <th className="px-4 py-1.5">Descripcion</th>
                        </tr>
                      </thead>
                      <tbody>
                        {forensicSec.events.slice(0, 50).map((evt, i) => (
                          <tr key={i} className="border-b border-neutral-100 hover:bg-neutral-50">
                            <td className="px-4 py-1.5 text-[10px] text-neutral-500 whitespace-nowrap">{evt.timestamp}</td>
                            <td className="px-4 py-1.5 text-[10px] font-mono">{evt.event_type}</td>
                            <td className="px-4 py-1.5"><SeverityBadge severity={evt.severity} /></td>
                            <td className="px-4 py-1.5 text-[10px] text-neutral-600 truncate max-w-[280px]">{evt.description}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </>
                ) : (
                  <div className="px-5 py-8 text-center">
                    <p className="text-sm text-neutral-400">
                      {forensicSec ? 'Sin eventos de seguridad — sistema limpio.' : 'Cargando...'}
                    </p>
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </div>

      {/* ── Service Detail Drawer ── */}
      <ServiceDrawer
        serviceName={drawerService}
        onClose={() => setDrawerService(null)}
        services={services}
        pentest={pentest}
        integrity={integrity}
        regulatory={regulatory}
        stress={stress}
        health={health}
        stats={stats}
        oracle={oracle}
        validators={validators}
      />
    </div>
  );
}

// ── Drawer ──────────────────────────────────────────────────────────────────

interface DrawerProps {
  serviceName: string | null;
  onClose: () => void;
  services: ServiceStatus[];
  pentest: PentestReport | null;
  integrity: IntegrityResult | null;
  regulatory: RegulatoryChecks | null;
  stress: StressReport | null;
  health: HealthResponse | null;
  stats: Stats | null;
  oracle: OracleStatus | null;
  validators: Validator[];
}

function ServiceDrawer({ serviceName, onClose, services, pentest, integrity, regulatory, stress, health, stats, oracle, validators }: DrawerProps) {
  if (!serviceName) return null;

  const svc = services.find((s) => s.name === serviceName);
  if (!svc) return null;
  const style = STATUS_STYLES[svc.status];

  return (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 z-50 bg-black/20" onClick={onClose} />

      {/* Drawer */}
      <div className="fixed inset-y-0 right-0 z-50 w-full max-w-lg bg-white shadow-xl flex flex-col animate-slide-in">
        {/* Header */}
        <div className={`px-5 py-4 border-b border-neutral-200 flex items-center justify-between ${style.bg}`}>
          <div>
            <h2 className="text-lg font-bold text-neutral-900">{svc.label}</h2>
            <div className="flex items-center gap-2 mt-0.5">
              <span className={`w-2 h-2 rounded-full ${style.dot}`} />
              <span className={`text-xs font-medium ${style.text}`}>{style.label}</span>
              <span className="text-[10px] text-neutral-400">· {svc.iso}</span>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-black/5 transition-colors"
            aria-label="Cerrar"
          >
            <svg className="w-5 h-5 text-neutral-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content — scrollable */}
        <div className="flex-1 overflow-y-auto px-5 py-4">
          {serviceName === 'security' && <SecurityDetail pentest={pentest} />}
          {serviceName === 'forensic' && <ForensicDetail integrity={integrity} />}
          {serviceName === 'compliance' && <ComplianceDetail regulatory={regulatory} />}
          {serviceName === 'crypto' && <StressModuleDetail stress={stress} moduleName="crypto_hash" />}
          {serviceName === 'storage' && <StorageDetail health={health} stats={stats} />}
          {serviceName === 'consensus' && <ConsensusDetail stats={stats} validators={validators} />}
          {serviceName === 'oracles' && <OracleDetail oracle={oracle} />}
          {serviceName === 'intelligence' && <IntelligenceDetail stress={stress} />}
        </div>
      </div>
    </>
  );
}

// ── Drawer detail views ─────────────────────────────────────────────────────

function SecurityDetail({ pentest }: { pentest: PentestReport | null }) {
  if (!pentest) return <DrawerEmpty />;
  const categories = [...new Set(pentest.results.map((r) => r.category))];
  return (
    <>
      <DrawerStats items={[
        { label: 'Total escenarios', value: pentest.total_scenarios },
        { label: 'Bloqueados', value: pentest.blocked, color: 'text-emerald-600' },
        { label: 'Detectados', value: pentest.detected, color: 'text-amber-600' },
        { label: 'Vulnerables', value: pentest.vulnerable, color: pentest.vulnerable > 0 ? 'text-red-600' : 'text-emerald-600' },
      ]} />
      <p className="text-[10px] text-neutral-400 mt-1 mb-3">Generado: {pentest.generated_at}</p>
      {categories.map((cat) => (
        <div key={cat} className="mb-3">
          <p className="text-[10px] font-bold text-neutral-400 uppercase tracking-widest mb-1.5">{cat}</p>
          {pentest.results.filter((r) => r.category === cat).map((r) => (
            <div key={r.id} className="flex items-start gap-2 py-1.5 border-b border-neutral-100">
              <span className={`mt-0.5 w-2 h-2 rounded-full flex-shrink-0 ${
                r.status === 'Blocked' ? 'bg-emerald-500' : r.status === 'Detected' ? 'bg-amber-500' : 'bg-red-500'
              }`} />
              <div className="min-w-0">
                <p className="text-xs font-medium text-neutral-800">{r.scenario}</p>
                <p className="text-[10px] text-neutral-500 truncate">{r.attack_vector}</p>
              </div>
              <span className={`ml-auto text-[10px] font-medium flex-shrink-0 ${
                r.status === 'Blocked' ? 'text-emerald-600' : r.status === 'Detected' ? 'text-amber-600' : 'text-red-600'
              }`}>{r.status}</span>
            </div>
          ))}
        </div>
      ))}
    </>
  );
}

function ForensicDetail({ integrity }: { integrity: IntegrityResult | null }) {
  if (!integrity) return <DrawerEmpty />;
  return (
    <>
      <DrawerStats items={[
        { label: 'Bloques verificados', value: integrity.blocks_checked },
        { label: 'Estado', value: integrity.status, color: integrity.status === 'Valid' ? 'text-emerald-600' : 'text-red-600' },
        { label: 'Inconsistencias', value: integrity.mismatches.length, color: integrity.mismatches.length > 0 ? 'text-red-600' : 'text-emerald-600' },
      ]} />
      {integrity.mismatches.length > 0 && (
        <div className="mt-4">
          <p className="text-[10px] font-bold text-neutral-400 uppercase tracking-widest mb-2">Inconsistencias detectadas</p>
          {integrity.mismatches.map((m, i) => (
            <div key={i} className="bg-red-50 rounded-lg px-3 py-2 mb-2 text-xs">
              <p className="font-medium text-red-800">Altura {m.height} — {m.field}</p>
              <p className="text-[10px] text-red-600 font-mono truncate">Esperado: {m.expected}</p>
              <p className="text-[10px] text-red-600 font-mono truncate">Actual: {m.actual}</p>
            </div>
          ))}
        </div>
      )}
      {integrity.mismatches.length === 0 && (
        <p className="mt-4 text-xs text-emerald-600">Cadena de hashes integra. Sin alteraciones detectadas.</p>
      )}
    </>
  );
}

function ComplianceDetail({ regulatory }: { regulatory: RegulatoryChecks | null }) {
  if (!regulatory) return <DrawerEmpty />;
  const categories = [...new Set(regulatory.checks.map((c) => c.category))];
  return (
    <>
      <DrawerStats items={[
        { label: 'Total checks', value: regulatory.summary.total },
        { label: 'Aprobados', value: regulatory.summary.passed, color: 'text-emerald-600' },
        { label: 'Fallidos', value: regulatory.summary.failed, color: regulatory.summary.failed > 0 ? 'text-red-600' : 'text-emerald-600' },
        { label: 'Tasa', value: `${regulatory.summary.pass_rate_pct.toFixed(1)}%` },
      ]} />
      {categories.map((cat) => (
        <div key={cat} className="mt-3">
          <p className="text-[10px] font-bold text-neutral-400 uppercase tracking-widest mb-1.5">{cat}</p>
          {regulatory.checks.filter((c) => c.category === cat).map((c) => (
            <div key={c.id} className="flex items-start gap-2 py-1.5 border-b border-neutral-100">
              <span className={`mt-0.5 w-2 h-2 rounded-full flex-shrink-0 ${
                c.status === 'Pass' ? 'bg-emerald-500' : c.status === 'Fail' ? 'bg-red-500' : 'bg-neutral-300'
              }`} />
              <div className="min-w-0">
                <p className="text-xs text-neutral-800">{c.description}</p>
                <p className="text-[10px] text-neutral-400 truncate">{c.evidence}</p>
              </div>
              <span className={`ml-auto text-[10px] font-medium flex-shrink-0 ${
                c.status === 'Pass' ? 'text-emerald-600' : c.status === 'Fail' ? 'text-red-600' : 'text-neutral-400'
              }`}>{c.status}</span>
            </div>
          ))}
        </div>
      ))}
    </>
  );
}

function StressModuleDetail({ stress, moduleName }: { stress: StressReport | null; moduleName: string }) {
  if (!stress) return <DrawerEmpty />;
  const m = stress.results.find((r) => r.module === moduleName);
  if (!m) return <p className="text-xs text-neutral-400">Modulo no encontrado.</p>;
  return (
    <>
      <DrawerStats items={[
        { label: 'Operaciones', value: m.operations.toLocaleString() },
        { label: 'Ops/s', value: m.ops_per_sec.toFixed(0) },
        { label: 'p50', value: `${m.p50_us}us` },
        { label: 'p99', value: `${m.p99_us}us` },
        { label: 'Errores', value: m.errors, color: m.errors > 0 ? 'text-red-600' : 'text-emerald-600' },
        { label: 'Duracion', value: `${m.duration_ms}ms` },
      ]} />
      <p className={`mt-3 text-xs font-medium ${m.status === 'Pass' ? 'text-emerald-600' : 'text-amber-600'}`}>
        Estado: {m.status}
      </p>
    </>
  );
}

function StorageDetail({ health, stats }: { health: HealthResponse | null; stats: Stats | null }) {
  if (!health) return <DrawerEmpty />;
  return (
    <>
      <DrawerStats items={[
        { label: 'Storage', value: health.checks?.storage || '?' },
        { label: 'Peers', value: health.checks?.peers || '?' },
        { label: 'Ordering', value: health.checks?.ordering || '?' },
        { label: 'Bloques', value: stats?.blockchain.block_count ?? '-' },
        { label: 'Transacciones', value: stats?.blockchain.total_transactions ?? '-' },
        { label: 'Uptime', value: formatUptime(health.uptime_seconds) },
      ]} />
      <p className={`mt-3 text-xs font-medium ${health.status === 'healthy' ? 'text-emerald-600' : 'text-amber-600'}`}>
        Estado general: {health.status}
      </p>
    </>
  );
}

function ConsensusDetail({ stats, validators }: { stats: Stats | null; validators: Validator[] }) {
  if (!stats) return <DrawerEmpty />;
  return (
    <>
      <DrawerStats items={[
        { label: 'Altura', value: stats.blockchain.latest_block_index },
        { label: 'Bloques', value: stats.blockchain.block_count },
        { label: 'Dificultad', value: stats.blockchain.difficulty },
        { label: 'Validadores', value: validators.length },
      ]} />
      {validators.length > 0 && (
        <div className="mt-4">
          <p className="text-[10px] font-bold text-neutral-400 uppercase tracking-widest mb-2">Validadores activos</p>
          {validators.map((v) => (
            <div key={v.address} className="flex items-center justify-between py-1.5 border-b border-neutral-100">
              <span className="text-xs font-mono text-neutral-700 truncate max-w-[200px]">{v.address}</span>
              <span className="text-[10px] text-neutral-500">{v.staked_amount.toLocaleString()} staked</span>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

function OracleDetail({ oracle }: { oracle: OracleStatus | null }) {
  if (!oracle) return <DrawerEmpty />;
  return (
    <DrawerStats items={[
      { label: 'Nodos', value: oracle.node_count },
      { label: 'Feeds', value: oracle.feed_count },
      { label: 'Frescos', value: oracle.fresh_feeds, color: 'text-emerald-600' },
      { label: 'Obsoletos', value: oracle.stale_feeds, color: oracle.stale_feeds > 0 ? 'text-amber-600' : 'text-emerald-600' },
      { label: 'Pendientes', value: oracle.pending_reports },
      { label: 'Max edad', value: `${(oracle.max_data_age_ms / 1000).toFixed(0)}s` },
    ]} />
  );
}

function IntelligenceDetail({ stress }: { stress: StressReport | null }) {
  if (!stress) return <DrawerEmpty />;
  const modules = stress.results.filter((r) =>
    ['risk_scoring', 'anomaly_detection', 'pattern_detection'].includes(r.module),
  );
  return (
    <>
      {modules.map((m) => (
        <div key={m.module} className="mb-4">
          <p className="text-xs font-semibold text-neutral-800 mb-1.5">{m.module.replace(/_/g, ' ')}</p>
          <DrawerStats items={[
            { label: 'Ops/s', value: m.ops_per_sec.toFixed(0) },
            { label: 'p50', value: `${m.p50_us}us` },
            { label: 'p99', value: `${m.p99_us}us` },
            { label: 'Errores', value: m.errors, color: m.errors > 0 ? 'text-red-600' : 'text-emerald-600' },
          ]} />
        </div>
      ))}
      {modules.length === 0 && <p className="text-xs text-neutral-400">Sin datos de modulos AML.</p>}
    </>
  );
}

// ── Shared drawer components ────────────────────────────────────────────────

function DrawerStats({ items }: { items: { label: string; value: string | number; color?: string }[] }) {
  return (
    <div className="grid grid-cols-3 gap-3 mb-2">
      {items.map((item) => (
        <div key={item.label}>
          <p className="text-[9px] text-neutral-400 uppercase">{item.label}</p>
          <p className={`text-sm font-bold ${item.color || 'text-neutral-800'}`}>{item.value}</p>
        </div>
      ))}
    </div>
  );
}

function DrawerEmpty() {
  return <p className="text-xs text-neutral-400">Cargando datos...</p>;
}

// ── Helpers ─────────────────────────────────────────────────────────────────

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function MiniSeverity({ label, value, color }: { label: string; value: number; color: string }) {
  const colors: Record<string, string> = {
    red: 'text-red-700',
    amber: 'text-amber-700',
    yellow: 'text-yellow-700',
    neutral: 'text-neutral-500',
  };
  return (
    <div className="text-center">
      <p className={`text-lg font-bold ${colors[color] || colors.neutral}`}>{value}</p>
      <p className="text-[9px] text-neutral-400 uppercase">{label}</p>
    </div>
  );
}

function SeverityBadge({ severity }: { severity: string }) {
  const styles: Record<string, string> = {
    Critical: 'bg-red-100 text-red-800',
    High: 'bg-amber-100 text-amber-800',
    Medium: 'bg-yellow-100 text-yellow-800',
    Low: 'bg-neutral-100 text-neutral-600',
  };
  return (
    <span className={`text-[10px] px-1.5 py-0.5 rounded-full font-medium ${styles[severity] || styles.Low}`}>
      {severity}
    </span>
  );
}
