/**
 * Rust Blockchain SDK - Main entry point
 * 
 * @packageDocumentation
 */

export { BlockchainClient } from './client';
export * from './types';

// Default export
import { BlockchainClient } from './client';
export default BlockchainClient;

