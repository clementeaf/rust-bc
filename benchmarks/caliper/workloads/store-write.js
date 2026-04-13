'use strict';

/**
 * Caliper workload module: store transaction writes.
 *
 * This is the adapter between Caliper and rust-bc's REST API.
 * Each submitTransaction call POSTs a store transaction.
 */

const https = require('https');

const agent = new https.Agent({ rejectUnauthorized: false });

class StoreWriteWorkload {
    constructor() {
        this.txIndex = 0;
        this.baseUrl = '';
    }

    async init(workerIndex, totalWorkers, roundIndex, args) {
        this.txIndex = workerIndex * 1000000; // Avoid ID collisions between workers
        this.baseUrl = args.nodeUrl || 'https://127.0.0.1:8080';
    }

    async submitTransaction() {
        this.txIndex++;
        const txId = `caliper-${this.txIndex}`;
        const now = Math.floor(Date.now() / 1000);

        const body = JSON.stringify({
            id: txId,
            block_height: 0,
            timestamp: now,
            input_did: 'did:bc:caliper-sender',
            output_recipient: 'did:bc:caliper-receiver',
            amount: 1,
            state: 'pending',
        });

        return new Promise((resolve, reject) => {
            const url = new URL(`${this.baseUrl}/api/v1/store/transactions`);
            const options = {
                hostname: url.hostname,
                port: url.port,
                path: url.pathname,
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Content-Length': Buffer.byteLength(body),
                    'X-Org-Id': 'org1',
                },
                agent,
                timeout: 10000,
            };

            const req = https.request(options, (res) => {
                let data = '';
                res.on('data', (chunk) => { data += chunk; });
                res.on('end', () => {
                    if (res.statusCode >= 200 && res.statusCode < 300) {
                        resolve({ status: 'success' });
                    } else {
                        resolve({ status: 'failed', error: `HTTP ${res.statusCode}` });
                    }
                });
            });

            req.on('error', (err) => resolve({ status: 'failed', error: err.message }));
            req.on('timeout', () => { req.destroy(); resolve({ status: 'failed', error: 'timeout' }); });
            req.write(body);
            req.end();
        });
    }

    async end() {}
}

function createWorkloadModule() {
    return new StoreWriteWorkload();
}

module.exports.createWorkloadModule = createWorkloadModule;
