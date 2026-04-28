import { unwrapGatewayData, isGatewayEnvelope } from '../src/envelope';

describe('unwrapGatewayData', () => {
  it('extracts data from gateway envelope', () => {
    const body = {
      status: 'Success',
      status_code: 200,
      data: { tx_id: 'tx-1', block_height: 3 },
      error: null,
      timestamp: '2026-04-07T00:00:00Z',
      trace_id: 'uuid',
    };
    const result = unwrapGatewayData<{ tx_id: string }>(body);
    expect(result.tx_id).toBe('tx-1');
  });

  it('extracts data from legacy envelope', () => {
    const body = {
      success: true,
      data: { address: '0xabc', balance: 100 },
    };
    const result = unwrapGatewayData<{ address: string }>(body);
    expect(result.address).toBe('0xabc');
  });

  it('throws on error envelope with message', () => {
    const body = {
      status: 'Error',
      status_code: 403,
      data: null,
      message: 'access denied',
    };
    expect(() => unwrapGatewayData(body)).toThrow('access denied');
  });

  it('throws on failed legacy envelope', () => {
    const body = { success: false, message: 'not found' };
    expect(() => unwrapGatewayData(body)).toThrow('not found');
  });

  it('throws on null body', () => {
    expect(() => unwrapGatewayData(null)).toThrow('Invalid API response body');
  });

  it('throws on non-object body', () => {
    expect(() => unwrapGatewayData('string')).toThrow('Invalid API response body');
  });
});

describe('isGatewayEnvelope', () => {
  it('returns true for gateway envelope', () => {
    expect(isGatewayEnvelope({ status: 'Success', data: {} })).toBe(true);
  });

  it('returns false for legacy envelope', () => {
    expect(isGatewayEnvelope({ success: true, data: {} })).toBe(false);
  });

  it('returns false for null', () => {
    expect(isGatewayEnvelope(null)).toBe(false);
  });
});
