import axios from 'axios';
import { BlockchainClient } from '../src/client';

// Mock axios entirely
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;

function envelope(data: unknown, statusCode = 200) {
  return {
    status: 'Success',
    status_code: statusCode,
    message: 'OK',
    data,
    error: null,
    timestamp: '2026-04-07T00:00:00Z',
    trace_id: 'test-trace',
  };
}

function legacyEnvelope(data: unknown) {
  return { success: true, data };
}

describe('BlockchainClient', () => {
  let client: BlockchainClient;
  let mockInstance: any;

  beforeEach(() => {
    mockInstance = {
      get: jest.fn(),
      post: jest.fn(),
      put: jest.fn(),
      defaults: { headers: { common: {} } },
    };
    mockedAxios.create.mockReturnValue(mockInstance);
    mockedAxios.isAxiosError.mockReturnValue(false);
    client = new BlockchainClient({ baseUrl: 'http://test/api/v1' });
  });

  // ── Health ───────────────────────────────────────────────────────────

  describe('health', () => {
    it('returns health check data', async () => {
      mockInstance.get.mockResolvedValue({
        data: envelope({
          status: 'healthy',
          uptime_seconds: 60,
          blockchain: { height: 5, last_block_hash: 'abc', validators_count: 0 },
          checks: { storage: 'ok', peers: 'ok (3)', ordering: 'ok' },
        }),
      });
      const health = await client.health();
      expect(health.status).toBe('healthy');
      expect(health.uptime_seconds).toBe(60);
    });
  });

  // ── Gateway ──────────────────────────────────────────────────────────

  describe('submitTransaction', () => {
    it('returns tx_id and block_height', async () => {
      mockInstance.post.mockResolvedValue({
        data: envelope({ tx_id: 'tx-001', block_height: 3, valid: true }),
      });
      const result = await client.submitTransaction('mycc', 'ch1', {
        id: 'tx-001',
        inputDid: 'did:bc:alice',
        outputRecipient: 'did:bc:bob',
        amount: 100,
      });
      expect(result.tx_id).toBe('tx-001');
      expect(result.block_height).toBe(3);
      expect(mockInstance.post).toHaveBeenCalledWith(
        '/gateway/submit',
        expect.objectContaining({ chaincode_id: 'mycc', channel_id: 'ch1' })
      );
    });
  });

  // ── Organizations ────────────────────────────────────────────────────

  describe('registerOrg', () => {
    it('registers and returns org', async () => {
      mockInstance.post.mockResolvedValue({
        data: envelope({ org_id: 'org1', name: 'Org 1', msp_id: 'Org1MSP' }),
      });
      const org = await client.registerOrg({
        org_id: 'org1',
        name: 'Org 1',
        msp_id: 'Org1MSP',
      });
      expect(org.org_id).toBe('org1');
    });
  });

  // ── Channels ─────────────────────────────────────────────────────────

  describe('createChannel', () => {
    it('creates channel', async () => {
      mockInstance.post.mockResolvedValue({
        data: envelope({ channel_id: 'mychannel' }),
      });
      const ch = await client.createChannel('mychannel');
      expect(ch.channel_id).toBe('mychannel');
    });
  });

  describe('listChannels', () => {
    it('lists channels', async () => {
      mockInstance.get.mockResolvedValue({
        data: envelope([{ channel_id: 'ch1' }, { channel_id: 'ch2' }]),
      });
      const channels = await client.listChannels();
      expect(channels).toHaveLength(2);
    });
  });

  // ── Private data ─────────────────────────────────────────────────────

  describe('putPrivateData', () => {
    it('writes private data with org header', async () => {
      mockInstance.put.mockResolvedValue({
        data: envelope({ collection: 'secret', key: 'k1', hash: 'abc' }),
      });
      const result = await client.putPrivateData('secret', 'k1', 'val', 'org1');
      expect(result.hash).toBe('abc');
      expect(mockInstance.put).toHaveBeenCalledWith(
        '/private-data/secret/k1',
        { value: 'val' },
        { headers: { 'X-Org-Id': 'org1' } }
      );
    });
  });

  describe('getPrivateData', () => {
    it('reads private data with org header', async () => {
      mockInstance.get.mockResolvedValue({
        data: envelope('secret-value'),
      });
      const value = await client.getPrivateData('secret', 'k1', 'org1');
      expect(value).toBe('secret-value');
    });
  });

  // ── Legacy endpoints ─────────────────────────────────────────────────

  describe('createWallet', () => {
    it('creates wallet from legacy envelope', async () => {
      mockInstance.post.mockResolvedValue({
        data: legacyEnvelope({ address: '0xabc', balance: 0, public_key: 'pk' }),
      });
      const wallet = await client.createWallet();
      expect(wallet.address).toBe('0xabc');
    });
  });

  describe('getBlocks', () => {
    it('returns block array', async () => {
      mockInstance.get.mockResolvedValue({
        data: envelope([{ index: 0, hash: 'genesis' }]),
      });
      const blocks = await client.getBlocks();
      expect(blocks).toHaveLength(1);
    });
  });

  describe('mineBlock', () => {
    it('mines and returns hash', async () => {
      mockInstance.post.mockResolvedValue({
        data: envelope({ hash: 'blockhash', reward: 50, transactions_count: 1 }),
      });
      const result = await client.mineBlock('miner1');
      expect(result.hash).toBe('blockhash');
    });
  });

  // ── Evaluate ─────────────────────────────────────────────────────────

  describe('evaluate', () => {
    it('simulates chaincode', async () => {
      mockInstance.post.mockResolvedValue({
        data: envelope({ result: 'ok', rwset: { reads: [], writes: [{ key: 'x', value: '1' }] } }),
      });
      const result = await client.evaluate('mycc', 'run', '1.0');
      expect(result.rwset.writes).toHaveLength(1);
    });
  });

  // ── setPolicy ────────────────────────────────────────────────────────

  describe('setPolicy', () => {
    it('calls POST /store/policies', async () => {
      mockInstance.post.mockResolvedValue({
        data: envelope({ resource_id: 'ch/cc' }),
      });
      await client.setPolicy('ch/cc', { NOutOf: { n: 2, orgs: ['org1', 'org2'] } });
      expect(mockInstance.post).toHaveBeenCalledWith(
        '/store/policies/ch/cc',
        { NOutOf: { n: 2, orgs: ['org1', 'org2'] } }
      );
    });
  });
});
