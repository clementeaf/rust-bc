import https from 'node:https'
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

/**
 * Dev server proxies /api to the rust-bc node. Docker uses HTTPS + self-signed certs on 8080;
 * the proxy must not verify TLS (`secure: false` + https.Agent). Plain HTTP: `http://127.0.0.1:8080`.
 */
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const apiTarget = env.VITE_API_PROXY_TARGET ?? 'http://127.0.0.1:8080'
  const isHttpsTarget = apiTarget.startsWith('https://')

  // Actix + rustls negotiate HTTP/2; some Node proxy clients break on h2 (TLS read errors). Force HTTP/1.1.
  const httpsAgent = new https.Agent({
    rejectUnauthorized: false,
    ALPNProtocols: ['http/1.1'],
  })

  return {
    plugins: [react(), tailwindcss()],
    server: {
      port: Number.parseInt(env.VITE_DEV_SERVER_PORT ?? '5173', 10),
      strictPort: false,
      proxy: {
        '/api': {
          target: apiTarget,
          changeOrigin: true,
          secure: false,
          agent: isHttpsTarget ? httpsAgent : undefined,
        },
      },
    },
  }
})
