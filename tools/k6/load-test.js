/**
 * Cerulean Ledger — k6 Load Test Suite
 *
 * Usage:
 *   k6 run tools/k6/load-test.js                          # default: 50 VUs, 2min
 *   k6 run tools/k6/load-test.js --env SCENARIO=spike      # spike test
 *   k6 run tools/k6/load-test.js --env SCENARIO=soak       # 10min sustained
 *   k6 run tools/k6/load-test.js --env SCENARIO=stress     # escalating to 300 VUs
 *
 * Thresholds:
 *   - p(95) response time < 500ms
 *   - error rate < 5%
 *   - health check p(99) < 200ms
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

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
const governanceLatency = new Trend('governance_latency', true);
const rateLimited = new Counter('rate_limited');

// ── Scenarios ────────────────────────────────────────────────────────────────

const scenarios = {
  default: {
    stages: [
      { duration: '30s', target: 15 },
      { duration: '1m', target: 15 },
      { duration: '30s', target: 0 },
    ],
  },
  spike: {
    stages: [
      { duration: '20s', target: 10 },
      { duration: '10s', target: 80 },
      { duration: '30s', target: 80 },
      { duration: '20s', target: 10 },
      { duration: '20s', target: 0 },
    ],
  },
  soak: {
    stages: [
      { duration: '1m', target: 15 },
      { duration: '8m', target: 15 },
      { duration: '1m', target: 0 },
    ],
  },
  stress: {
    stages: [
      { duration: '30s', target: 15 },
      { duration: '30s', target: 30 },
      { duration: '30s', target: 60 },
      { duration: '30s', target: 100 },
      { duration: '1m', target: 0 },
    ],
  },
};

export const options = {
  stages: scenarios[SCENARIO]?.stages || scenarios.default.stages,
  thresholds: {
    http_req_duration: ['p(95)<500'],
    errors: ['rate<0.05'],
    health_latency: ['p(99)<200'],
  },
};

// ── Test Functions ───────────────────────────────────────────────────────────

function isRateLimited(res) {
  if (res.status === 429) {
    rateLimited.add(1);
    return true;
  }
  return false;
}

function testHealth() {
  const res = http.get(`${API}/health`);
  healthLatency.add(res.timings.duration);
  const ok = check(res, {
    'health: status 200': (r) => r.status === 200,
    'health: is healthy': (r) => {
      try { return JSON.parse(r.body).data.status === 'healthy'; }
      catch { return false; }
    },
  });
  errorRate.add(!ok);
}

function testStats() {
  const res = http.get(`${API}/stats`);
  if (isRateLimited(res)) return;
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
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'blocks: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testCreateIdentity() {
  const slug = `k6-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const now = Math.floor(Date.now() / 1000);
  const payload = JSON.stringify({
    did: `did:cerulean:${slug}`,
    created_at: now,
    updated_at: now,
    status: 'active',
  });
  const res = http.post(`${API}/store/identities`, payload, { headers: HEADERS });
  if (isRateLimited(res)) return;
  identityLatency.add(res.timings.duration);
  const ok = check(res, {
    'identity: status 2xx': (r) => r.status >= 200 && r.status < 300,
  });
  errorRate.add(!ok);
}

function testListIdentities() {
  const res = http.get(`${API}/store/identities`, { headers: HEADERS });
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'list-identities: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testListCredentials() {
  const res = http.get(`${API}/store/credentials`, { headers: HEADERS });
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'list-credentials: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testGovernance() {
  const res = http.get(`${API}/governance/proposals`);
  if (isRateLimited(res)) return;
  governanceLatency.add(res.timings.duration);
  const ok = check(res, {
    'governance: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testGovernanceParams() {
  const res = http.get(`${API}/governance/params`);
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'params: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testRegulatoryChecks() {
  const res = http.get(`${API}/regulatory/checks`);
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'regulatory: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testCreateWalletAndMine() {
  const res = http.post(`${API}/wallets/create`, '{}', { headers: HEADERS });
  if (isRateLimited(res)) return;
  let addr = null;
  const ok1 = check(res, {
    'wallet: status 2xx': (r) => r.status >= 200 && r.status < 300,
    'wallet: has address': (r) => {
      try { addr = JSON.parse(r.body).data.address; return addr.length > 0; }
      catch { return false; }
    },
  });
  errorRate.add(!ok1);

  if (addr) {
    const mineRes = http.post(`${API}/mine`, JSON.stringify({ miner_address: addr }), { headers: HEADERS });
    if (!isRateLimited(mineRes)) {
      mineLatency.add(mineRes.timings.duration);
      const ok2 = check(mineRes, {
        'mine: status 2xx': (r) => r.status >= 200 && r.status < 300,
      });
      errorRate.add(!ok2);
    }
  }
}

function testAuditQuery() {
  const res = http.get(`${API}/audit/requests?limit=10`);
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'audit: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testMempool() {
  const res = http.get(`${API}/mempool`);
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'mempool: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testOracleStatus() {
  const res = http.get(`${API}/oracle/status`);
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'oracle: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testForensicIntegrity() {
  const res = http.get(`${API}/forensic/integrity`);
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'forensic: status 200': (r) => r.status === 200,
  });
  errorRate.add(!ok);
}

function testSubmitProposal() {
  const payload = JSON.stringify({
    proposer: `k6-proposer-${__VU}`,
    description: `Load test proposal from VU ${__VU} iter ${__ITER}`,
    deposit: 10000,
    action: { type: 'text', title: `k6-proposal-${Date.now()}`, description: 'Load test' },
  });
  const res = http.post(`${API}/governance/proposals`, payload, { headers: HEADERS });
  if (isRateLimited(res)) return;
  // 400 = governance rate limit (1 per 100 blocks per proposer) — expected, not an error
  const ok = check(res, {
    'proposal: not server error': (r) => r.status < 500,
  });
  errorRate.add(res.status >= 500);
}

function testCastVote() {
  const payload = JSON.stringify({
    voter: `k6-voter-${__VU}-${__ITER}`,
    option: ['Yes', 'No', 'Abstain'][Math.floor(Math.random() * 3)],
  });
  // Vote on proposal 1 (seeded)
  const res = http.post(`${API}/governance/proposals/1/vote`, payload, { headers: HEADERS });
  if (isRateLimited(res)) return;
  // 400 = already voted or proposal not found — not a server error
  const ok = check(res, {
    'vote: not server error': (r) => r.status < 500,
  });
  errorRate.add(!ok);
}

function testIssueCredential() {
  // Create issuer identity first, then credential
  const slug = `k6-issuer-${__VU}-${__ITER}-${Date.now()}`;
  const now = Math.floor(Date.now() / 1000);
  const issuerDid = `did:cerulean:${slug}`;

  // Register issuer
  http.post(`${API}/store/identities`, JSON.stringify({
    did: issuerDid, created_at: now, updated_at: now, status: 'active',
  }), { headers: HEADERS });

  // Issue credential
  const credPayload = JSON.stringify({
    id: `k6-cred-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    issuer_did: issuerDid,
    subject_did: `did:cerulean:k6-subject-${__VU}`,
    cred_type: 'LoadTestCredential',
    issued_at: now,
    expires_at: 0,
    claims: { test: true, vu: __VU },
    status: 'active',
  });
  const res = http.post(`${API}/store/credentials`, credPayload, { headers: HEADERS });
  if (isRateLimited(res)) return;
  const ok = check(res, {
    'credential: status 2xx': (r) => r.status >= 200 && r.status < 300,
  });
  errorRate.add(!ok);
}

// ── Main Loop ────────────────────────────────────────────────────────────────

export default function () {
  const roll = Math.random();

  // 40% reads, 40% mutations, 20% heavy ops — realistic institutional mix
  if (roll < 0.08) {
    group('health', testHealth);
  } else if (roll < 0.14) {
    group('stats', testStats);
  } else if (roll < 0.20) {
    group('blocks', testBlocks);
  } else if (roll < 0.25) {
    group('mempool', testMempool);
  } else if (roll < 0.30) {
    group('audit', testAuditQuery);
  } else if (roll < 0.35) {
    group('identity-list', testListIdentities);
  } else if (roll < 0.40) {
    group('credential-list', testListCredentials);
  } else if (roll < 0.48) {
    group('identity-create', testCreateIdentity);
  } else if (roll < 0.56) {
    group('credential-issue', testIssueCredential);
  } else if (roll < 0.62) {
    group('proposal-submit', testSubmitProposal);
  } else if (roll < 0.70) {
    group('vote-cast', testCastVote);
  } else if (roll < 0.75) {
    group('governance-read', testGovernance);
  } else if (roll < 0.80) {
    group('governance-params', testGovernanceParams);
  } else if (roll < 0.84) {
    group('oracle-status', testOracleStatus);
  } else if (roll < 0.88) {
    group('forensic', testForensicIntegrity);
  } else if (roll < 0.92) {
    group('regulatory', testRegulatoryChecks);
  } else if (roll < 0.97) {
    group('wallet+mine', testCreateWalletAndMine);
  } else {
    group('wallet', () => {
      const res = http.post(`${API}/wallets/create`, '{}', { headers: HEADERS });
      if (!isRateLimited(res)) {
        errorRate.add(!check(res, { 'wallet: status 2xx': (r) => r.status >= 200 && r.status < 300 }));
      }
    });
  }

  sleep(0.3 + Math.random() * 0.5); // 300-800ms think time (realistic user)
}

// ── Summary ──────────────────────────────────────────────────────────────────

export function handleSummary(data) {
  const now = new Date().toISOString().replace(/[:.]/g, '-');
  return {
    stdout: textSummary(data, { indent: '  ', enableColors: true }),
    [`tools/k6/results/report-${now}.json`]: JSON.stringify(data, null, 2),
  };
}

import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';
