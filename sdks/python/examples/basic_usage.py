"""Basic usage example for the rust-bc Python SDK."""

from rust_bc import BlockchainClient, TransactionInput

# Connect to a local node (skip TLS verification for development)
client = BlockchainClient(
    "https://localhost:8080/api/v1",
    verify_ssl=False,
)

# Check health
health = client.health()
print(f"Status: {health.status}")
print(f"Block height: {health.blockchain.height}")
if health.checks:
    print(f"Storage: {health.checks.storage}")
    print(f"Peers: {health.checks.peers}")

# Submit a transaction via gateway
tx = TransactionInput(
    id="tx-python-001",
    input_did="did:bc:alice",
    output_recipient="did:bc:bob",
    amount=100,
)
result = client.submit_transaction("mycc", "", tx)
print(f"TX committed: {result.tx_id} at block {result.block_height}")

# Create a channel
channel = client.create_channel("test-channel")
print(f"Channel created: {channel.channel_id}")

# Chain info
info = client.chain_info()
print(f"Block count: {info.get('block_count')}")

client.close()
