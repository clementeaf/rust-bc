/**
 * Cerulean Ledger — E2E Institutional Flow Test
 *
 * Tests the complete lifecycle that the Blockchain Chamber will exercise:
 *   1. Create institution identity
 *   2. Create person identity
 *   3. Issue credential (institution → person)
 *   4. Verify credential exists
 *   5. Submit governance proposal
 *   6. Cast vote on proposal
 *   7. Check tally
 *   8. Query audit trail
 *   9. Check regulatory compliance
 *  10. Verify platform integrity
 *
 * Usage:
 *   k6 run tools/k6/e2e-flow.js
 *   k6 run tools/k6/e2e-flow.js --env BASE_URL=http://localhost:9600
 */

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const BASE = __ENV.BASE_URL || 'http://127.0.0.1:8080';
const API = `${BASE}/api/v1`;

const HEADERS = {
  'Content-Type': 'application/json',
  'X-Org-Id': 'e2e-test-org',
  'X-Msp-Role': 'admin',
};

const flowErrors = new Rate('flow_errors');
const flowLatency = new Trend('flow_total_latency', true);

export const options = {
  scenarios: {
    e2e_flow: {
      executor: 'per-vu-iterations',
      vus: 5,
      iterations: 10,
      maxDuration: '5m',
    },
  },
  thresholds: {
    flow_errors: ['rate<0.05'],
    flow_total_latency: ['p(95)<5000'],
  },
};

export default function () {
  const flowStart = Date.now();
  const uid = `${__VU}-${__ITER}-${Date.now()}`;

  let ok;

  // ── Step 1: Create institution ─────────────────────────────────────────
  const instDid = `did:cerulean:e2e-inst-${uid}`;
  const now = Math.floor(Date.now() / 1000);

  ok = group('1-create-institution', () => {
    const res = http.post(`${API}/store/identities`, JSON.stringify({
      did: instDid, created_at: now, updated_at: now, status: 'active',
    }), { headers: HEADERS });
    return check(res, {
      'institution created': (r) => r.status >= 200 && r.status < 300,
    });
  });
  flowErrors.add(!ok);
  if (!ok) return;

  sleep(0.1);

  // ── Step 2: Create person ──────────────────────────────────────────────
  const personDid = `did:cerulean:e2e-person-${uid}`;

  ok = group('2-create-person', () => {
    const res = http.post(`${API}/store/identities`, JSON.stringify({
      did: personDid, created_at: now, updated_at: now, status: 'active',
    }), { headers: HEADERS });
    return check(res, {
      'person created': (r) => r.status >= 200 && r.status < 300,
    });
  });
  flowErrors.add(!ok);
  if (!ok) return;

  sleep(0.1);

  // ── Step 3: Issue credential ───────────────────────────────────────────
  const credId = `e2e-cred-${uid}`;

  ok = group('3-issue-credential', () => {
    const res = http.post(`${API}/store/credentials`, JSON.stringify({
      id: credId,
      issuer_did: instDid,
      subject_did: personDid,
      cred_type: 'Titulo Profesional',
      issued_at: now,
      expires_at: 0,
      claims: { grado: 'Ingenieria', test: uid },
      status: 'active',
    }), { headers: HEADERS });
    return check(res, {
      'credential issued': (r) => r.status >= 200 && r.status < 300,
    });
  });
  flowErrors.add(!ok);
  if (!ok) return;

  sleep(0.1);

  // ── Step 4: Verify credential ──────────────────────────────────────────
  ok = group('4-verify-credential', () => {
    const res = http.get(`${API}/store/credentials/${encodeURIComponent(credId)}`, { headers: HEADERS });
    return check(res, {
      'credential found': (r) => r.status === 200,
      'credential data matches': (r) => {
        try {
          const d = JSON.parse(r.body).data;
          return d.id === credId && d.issuer_did === instDid && d.subject_did === personDid;
        } catch { return false; }
      },
    });
  });
  flowErrors.add(!ok);

  sleep(0.1);

  // ── Step 5: Submit governance proposal ─────────────────────────────────
  let proposalId = null;

  ok = group('5-submit-proposal', () => {
    const res = http.post(`${API}/governance/proposals`, JSON.stringify({
      proposer: `e2e-proposer-${uid}`,
      description: `E2E test proposal ${uid}`,
      deposit: 10000,
      action: { type: 'text', title: `E2E proposal ${uid}`, description: 'Automated E2E test' },
    }), { headers: HEADERS });
    const passed = check(res, {
      'proposal submitted': (r) => r.status >= 200 && r.status < 300,
    });
    if (passed) {
      try { proposalId = JSON.parse(res.body).data.id; } catch {}
    }
    return passed;
  });
  flowErrors.add(!ok);

  sleep(0.1);

  // ── Step 6: Cast vote ──────────────────────────────────────────────────
  if (proposalId) {
    ok = group('6-cast-vote', () => {
      const res = http.post(`${API}/governance/proposals/${proposalId}/vote`, JSON.stringify({
        voter: `e2e-voter-${uid}`,
        option: 'Yes',
      }), { headers: HEADERS });
      return check(res, {
        'vote cast': (r) => r.status >= 200 && r.status < 300,
      });
    });
    flowErrors.add(!ok);

    sleep(0.1);

    // ── Step 7: Check tally ──────────────────────────────────────────────
    ok = group('7-check-tally', () => {
      const res = http.get(`${API}/governance/proposals/${proposalId}/tally`);
      return check(res, {
        'tally returned': (r) => r.status === 200,
        'tally has votes': (r) => {
          try { return JSON.parse(r.body).data.total_voted_power > 0; }
          catch { return false; }
        },
      });
    });
    flowErrors.add(!ok);
  }

  sleep(0.1);

  // ── Step 8: Query audit trail ──────────────────────────────────────────
  ok = group('8-audit-trail', () => {
    const res = http.get(`${API}/audit/requests?limit=5`);
    return check(res, {
      'audit returns': (r) => r.status === 200,
    });
  });
  flowErrors.add(!ok);

  sleep(0.1);

  // ── Step 9: Regulatory compliance ──────────────────────────────────────
  ok = group('9-regulatory', () => {
    const res = http.get(`${API}/regulatory/checks`);
    return check(res, {
      'regulatory returns': (r) => r.status === 200,
      'regulatory has checks': (r) => {
        try { return JSON.parse(r.body).data.summary.total > 0; }
        catch { return false; }
      },
    });
  });
  flowErrors.add(!ok);

  sleep(0.1);

  // ── Step 10: Platform health ───────────────────────────────────────────
  ok = group('10-platform-health', () => {
    const res = http.get(`${API}/health`);
    return check(res, {
      'platform healthy': (r) => {
        try { return JSON.parse(r.body).data.status === 'healthy'; }
        catch { return false; }
      },
    });
  });
  flowErrors.add(!ok);

  flowLatency.add(Date.now() - flowStart);
}

export function handleSummary(data) {
  const now = new Date().toISOString().replace(/[:.]/g, '-');
  return {
    stdout: textSummary(data, { indent: '  ', enableColors: true }),
    [`tools/k6/results/e2e-${now}.json`]: JSON.stringify(data, null, 2),
  };
}

import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';
