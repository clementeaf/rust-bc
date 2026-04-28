/**
 * Gateway (NeuroAccess-style) responses use `status`, `data`, `error`, `trace_id`.
 * Legacy handlers still return `{ success, data?, message? }` until migrated.
 */

/**
 * Extracts the successful payload from a gateway or legacy JSON body.
 * @param body - Parsed JSON response body
 * @returns The inner `data` payload on success
 */
export function unwrapGatewayData<T>(body: unknown): T {
  if (body === null || typeof body !== 'object') {
    throw new Error('Invalid API response body');
  }
  const record = body as Record<string, unknown>;
  if (
    record.status === 'Success' &&
    'data' in record &&
    record.data !== undefined &&
    record.data !== null
  ) {
    return record.data as T;
  }
  if (record.success === true && 'data' in record && record.data !== undefined) {
    return record.data as T;
  }
  let message = 'Request failed';
  if (typeof record.message === 'string' && record.message.length > 0) {
    message = record.message;
  } else if (
    record.error !== null &&
    typeof record.error === 'object' &&
    record.error !== undefined &&
    'message' in record.error &&
    typeof (record.error as { message: unknown }).message === 'string'
  ) {
    message = (record.error as { message: string }).message;
  }
  throw new Error(message);
}

/**
 * True if the body uses the gateway envelope (not legacy-only).
 */
export function isGatewayEnvelope(body: unknown): boolean {
  return (
    typeof body === 'object' &&
    body !== null &&
    'status' in body &&
    typeof (body as { status: unknown }).status === 'string'
  );
}
