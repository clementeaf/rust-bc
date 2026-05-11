/**
 * Cerulean Ledger — k6 Load Test Suite
 *
 * Usage:
 *   k6 run tools/k6/load-test.js                          # default: 50 VUs, 2min
 *   k6 run tools/k6/load-test.js --env BASE_URL=https://localhost:8080
 *   k6 run tools/k6/load-test.js --env SCENARIO=spike      # spike test
 *   k6 run tools/k6/load-test.js --env SCENARIO=soak       # 10min sustained
 *
 * Prerequisites:
 *   - Node running at BASE_URL (default: http://127.0.0.1:8080)
 *   - ACL_MODE=permissive (or valid X-Org-Id / TLS cert)
 *   - k6 installed: https://k6.io/docs/get-started/installation/
 *
 * Thresholds (fail the test if exceeded):
 *   - p(95) response time < 500ms
 *   - error rate < 1%
 *   - health check p(99) < 100ms
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// ── Configuration ────────────────────────────────────────────────────────────

const BASE = __ENV.BASE_URL || 'http://127.0.0.1:8080';
const API = `${BASE}/api/v1`;
const SCENARIO = __ENV.SCENARIO || 'default';

const HEADERS = {
  'Content-Type': 'application/json',
  'X-Org-Id': 'loadtest-org',
  'X-Msp-Role': 'admin',
};

// Custom metrics
const errorRate = new Rate('errors');
const healthLatency = new Trend('health_latency', true);
const mineLatency = new Trend('mine_latency', true);
const identityLatency = new Trend('identity_latency', true);

// ── Scenarios ────────────────────────────────────────────────────────────────

const scenarios = {
  default: {
    stages: [
      { duration: '30s', target: 20 },  // ramp up
      { duration: '1m', target: 50 },   // sustained
      { duration: '30s', target: 0 },   // ramp down
    ],
  },
  spike: {
    stages: [
      { duration: '20s', target: 10 },  // warm up
      { duration: '10s', target: 200 }, // spike
      { duration: '30s', target: 200 }, // hold spike
      { duration: '20s', target: 10 },  // recovery
      { duration: '20s', target: 0 },   // ramp down
    ],
  },
  soak: {
    stages: [
      { duration: '1m', target: 30 },   // ramp up
      { duration: '8m', target: 30 },   // sustained soak
      { duration: '1m', target: 0 },    // ramp down
    ],
  },
  stress: {
    stages: [
      { duration: '30s', target: 50 },
      { duration: '30s', target: 100 },
      { duration: '30s', target: 200 },
      { duration: '30s', target: 300 },
      { duration: '1m', target: 0 },
    ],
  },
};

export const options = {
  stages: scenarios[SCENARIO]?.stages || scenarios.default.stages,
  thresholds: {
    http_req_duration: ['p(95)<500'],
    errors: ['rate<0.01'],
    health_latency: ['p(99)<100'],
  },
};

// ── Test Functions ───────────────────────────────────────────────────────────

function testHealth() {
  const res = http.get(`${API}/health`);
  healthLatency.add(res.timings.duration);
  const ok = check(res, {
    'health: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testStats() {
  const res = http.get(`${API}/stats`);
  const ok = check(res, {
    'stats: status 200': (r) => r.status === 200,
    'stats: has block_count': (r) => {
      try { return JSON.parse(r.body).data.blockchain.block_count >= 0; }
      catch { return false; }
    },
  });
  errorRate.add(!ok);
}

function testBlocks() {
  const res = http.get(`${API}/blocks`);
  const ok = check(res, {
    'blocks: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testCreateWallet() {
  const res = http.post(`${API}/wallets/create`, '{}', { headers: HEADERS });
  const ok = check(res, {
    'wallet: status 2xx': (r) => r.status >= 200 && r.status < 300,
    'wallet: has address': (r) => {
      try { return JSON.parse(r.body).data.address.length > 0; }
      catch { return false; }
    },
  });
  errorRate.add(!ok);
  return ok ? JSON.parse(res.body).data.address : null;
}

function testMineBlock(minerAddress) {
  if (!minerAddress) return;
  const payload = JSON.stringify({ miner_address: minerAddress });
  const res = http.post(`${API}/mine`, payload, { headers: HEADERS });
  mineLatency.add(res.timings.duration);
  const ok = check(res, {
    'mine: status 2xx': (r) => r.status >= 200 && r.status < 300,
  });
  errorRate.add(!ok);
}

function testCreateIdentity() {
  const payload = JSON.stringify({ metadata: {} });
  const res = http.post(`${API}/identity/create`, payload, { headers: HEADERS });
  identityLatency.add(res.timings.duration);
  const ok = check(res, {
    'identity: status 2xx': (r) => r.status >= 200 && r.status < 300,
    'identity: has DID': (r) => {
      try { return JSON.parse(r.body).data.did.startsWith('did:cerulean:'); }
      catch { return false; }
    },
  });
  errorRate.add(!ok);
}

function testAuditQuery() {
  const res = http.get(`${API}/audit/requests?limit=10`);
  const ok = check(res, {
    'audit: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testMempool() {
  const res = http.get(`${API}/mempool`);
  const ok = check(res, {
    'mempool: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

// ── Main Loop ────────────────────────────────────────────────────────────────

export default function () {
  // Each VU iteration picks a weighted random action.
  // Read-heavy distribution mimics real traffic patterns.
  const roll = Math.random();

  if (roll < 0.25) {
    group('health', testHealth);
  } else if (roll < 0.40) {
    group('stats', testStats);
  } else if (roll < 0.55) {
    group('blocks', testBlocks);
  } else if (roll < 0.65) {
    group('mempool', testMempool);
  } else if (roll < 0.75) {
    group('audit', testAuditQuery);
  } else if (roll < 0.85) {
    group('identity', testCreateIdentity);
  } else if (roll < 0.93) {
    group('wallet+mine', () => {
      const addr = testCreateWallet();
      if (addr) testMineBlock(addr);
    });
  } else {
    group('wallet', testCreateWallet);
  }

  sleep(0.1 + Math.random() * 0.3); // 100-400ms think time
}

// ── Summary ──────────────────────────────────────────────────────────────────

export function handleSummary(data) {
  const now = new Date().toISOString().replace(/[:.]/g, '-');
  return {
    stdout: textSummary(data, { indent: '  ', enableColors: true }),
    [`tools/k6/results/report-${now}.json`]: JSON.stringify(data, null, 2),
  };
}

// k6 built-in text summary (available in k6 ≥ 0.30)
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';
