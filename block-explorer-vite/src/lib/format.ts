/** Tiempo relativo en español desde un timestamp Unix (segundos). */
export function timeAgo(ts: number): string {
  if (!ts) return '—'
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 0) return 'reciente'
  if (diff < 60) return `hace ${diff}s`
  if (diff < 3600) return `hace ${Math.floor(diff / 60)} min`
  if (diff < 86400) return `hace ${Math.floor(diff / 3600)} h`
  return `hace ${Math.floor(diff / 86400)} d`
}

/** Acorta un hash o direccion larga para mostrar en tablas. */
export function shortHash(h: string): string {
  return h.length > 16 ? h.slice(0, 8) + '...' + h.slice(-8) : h
}

/** Acorta un DID u otro codigo largo (mantiene mas caracteres). */
export function shortCode(d: string): string {
  return d.length > 24 ? d.slice(0, 12) + '...' + d.slice(-12) : d
}

/** Fecha legible en español desde timestamp Unix (segundos). */
export function fmtDate(ts: number): string {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleDateString('es-CL', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

/** Describe cuanto falta o cuanto paso desde un timestamp (para expiraciones). */
export function formatExpiry(ts: number): string {
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
