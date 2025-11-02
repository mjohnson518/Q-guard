# Q-guard

Production-ready Ethereum analytics API monetized via x402 micropayments on Base Sepolia testnet.

## Overview

Q-guard is an experimental Ethereum analytics API demonstrating the x402 micropayment protocol. It provides real-time gas predictions through a pay-per-request model using USDC on Base Sepolia testnet.

**Key Features:**
- Real-time gas prediction with exponential weighted average algorithm
- x402 micropayment integration ($0.01 USDC per request)
- Payment verification via onchain USDC transactions
- Redis caching for < 200ms response times
- WebSocket dashboard for real-time monitoring
- Docker deployment ready

**Tech Stack:**
- **Language:** Rust 2021
- **Framework:** Axum 0.7
- **Blockchain:** Ethereum mainnet (data), Base Sepolia (payments)
- **Payment:** x402 protocol with USDC
- **Cache:** Redis with moka in-memory fallback

## Prerequisites

- Rust 1.75+
- Docker & Docker Compose
- Base Sepolia ETH (for gas fees)
- Base Sepolia USDC (for testing payments)

## Quick Start

### 1. Clone Repository

```bash
git clone https://github.com/mjohnson518/Q-guard.git
cd Q-guard
```

### 2. Configure Environment

Copy the example environment file and fill in your credentials:

```bash
cp env.example .env
```

**Required Environment Variables:**

```env
# Ethereum Mainnet (for data)
ETH_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
ETH_RPC_FALLBACK=https://mainnet.infura.io/v3/YOUR_BACKUP_KEY

# Base Sepolia (for payments)
BASE_SEPOLIA_RPC_URL=https://base-sepolia.g.alchemy.com/v2/YOUR_KEY
USDC_ADDRESS=0x036CbD53842c5426634e7929541eC2318f3dCF7e

# Your Wallet
RECIPIENT_ADDRESS=0xYourBaseSepoliaAddress
SELLER_PRIVATE_KEY=0xYourPrivateKey

# Testing
TEST_WALLET_ADDRESS=0xYourTestWalletAddress
TEST_WALLET_PRIVATE_KEY=0xYourTestWalletPrivateKey
```

### 3. Run with Docker

```bash
docker-compose up -d
```

The API will be available at `http://localhost:8080`

### 4. Run Locally

```bash
# Start Redis
docker-compose up -d redis

# Run the server
cargo run

# In another terminal, test the payment flow
cargo run --bin test-agent
```

## API Endpoints

### Public Endpoints (No Payment Required)

#### Health Check
```bash
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "redis": true,
  "ethereum_rpc": true,
  "uptime_seconds": 3600,
  "timestamp": "2025-11-02T10:30:00Z"
}
```

#### Stats
```bash
GET /stats
```

**Response:**
```json
{
  "total_payments": 42,
  "revenue_today_usd": 0.42,
  "requests_today": 42,
  "cache_hit_rate": 0.85,
  "avg_response_time_ms": 150.5
}
```

#### WebSocket Dashboard
```bash
WS /ws/dashboard
```

Real-time stats updates every second.

### Protected Endpoints (Payment Required)

#### Gas Prediction ($0.01 USDC)

**Without Payment:**
```bash
GET /api/gas/prediction
```

**Response (402 Payment Required):**
```json
{
  "success": false,
  "error": "Payment required: 0.01 USDC",
  "error_code": "PAYMENT_REQUIRED",
  "timestamp": "2025-11-02T10:30:00Z",
  "request_id": "uuid-here",
  "payment_instructions": {
    "type": "x402.payment_required",
    "version": "1.0.0",
    "payment": {
      "chain": "base-sepolia",
      "asset": "USDC",
      "amount": "0.01",
      "recipient": "0xYourAddress",
      "facilitator": "https://x402-facilitator.example.com"
    },
    "instructions": {
      "header": "X-Payment",
      "format": "transaction_hash"
    }
  }
}
```

**With Payment:**
```bash
curl -H "X-Payment: 0x<transaction_hash>" \
  http://localhost:8080/api/gas/prediction
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "base_fee_gwei": 25.3,
    "priority_fee_gwei": 2.0,
    "max_fee_gwei": 32.36,
    "confidence": 0.92,
    "block_number": 18500000,
    "predicted_at": "2025-11-02T10:30:00Z",
    "next_block_time_seconds": 12
  },
  "timestamp": "2025-11-02T10:30:00Z",
  "cache_hit": true,
  "data_source": "ethereum-mainnet",
  "request_id": "uuid-here"
}
```

## x402 Payment Flow

### For API Consumers

1. **Make Initial Request**
   ```bash
   GET /api/gas/prediction
   ```

2. **Receive 402 Payment Required**
   - Contains payment instructions
   - Specifies amount, recipient, and payment network

3. **Send USDC Payment**
   - Transfer required USDC amount to recipient address on Base Sepolia
   - Wait for transaction confirmation

4. **Retry with Payment Proof**
   ```bash
   curl -H "X-Payment: 0x<tx_hash>" /api/gas/prediction
   ```

5. **Receive Data**
   - Payment is verified onchain
   - Request processed and data returned

### Payment Verification Process

```
┌────────┐         ┌─────────┐         ┌──────────────┐
│ Client │         │ Q-guard │         │ Base Sepolia │
└───┬────┘         └────┬────┘         └──────┬───────┘
    │                   │                     │
    │ GET /api/gas      │                     │
    ├──────────────────►│                     │
    │                   │                     │
    │ 402 Payment Req   │                     │
    │◄──────────────────┤                     │
    │                   │                     │
    │ Send USDC tx      │                     │
    ├───────────────────┼────────────────────►│
    │                   │                     │
    │ tx hash           │                     │
    │◄──────────────────┼─────────────────────┤
    │                   │                     │
    │ GET + X-Payment   │                     │
    ├──────────────────►│                     │
    │                   │                     │
    │                   │ Verify tx           │
    │                   ├────────────────────►│
    │                   │                     │
    │                   │ tx confirmed        │
    │                   │◄────────────────────┤
    │                   │                     │
    │ 200 OK + data     │                     │
    │◄──────────────────┤                     │
    │                   │                     │
```

## Testing

### Test Agent

The test agent performs a complete payment flow:

```bash
cargo run --bin test-agent
```

**What it does:**
1. Checks your USDC balance
2. Makes initial request (expects 402)
3. Sends USDC payment on Base Sepolia
4. Retries with payment proof
5. Displays gas prediction data

**Example Output:**
```
Q-guard Test Agent
===================
Server: http://localhost:8080
Recipient: 0x1234...

Your USDC balance: 1.000000

Testing payment flow...

Step 1: Making initial request (expecting 402)...
   [OK] Received 402 Payment Required

Step 2: Sending USDC payment on Base Sepolia...
   [OK] Payment sent: 0xabc123...
   View on BaseScan: https://sepolia.basescan.org/tx/0xabc123...

Step 3: Retrying request with payment proof...
   [OK] Payment verified!

[SUCCESS] Received gas prediction:
{
  "success": true,
  "data": {
    "base_fee_gwei": 25.3,
    "priority_fee_gwei": 2.0,
    "max_fee_gwei": 32.36,
    "confidence": 0.92,
    ...
  }
}
```

## Architecture

### Gas Prediction Algorithm

Q-guard uses an exponential weighted average algorithm to predict next-block gas prices:

1. **Fetch last 20 blocks** from Ethereum mainnet
2. **Apply exponential weights** (more recent = higher weight)
3. **Calculate weighted average** of base fees
4. **Add 20% safety buffer** for max fee
5. **Compute confidence score** based on variance

```rust
// Recent blocks weighted more heavily
weights = [0.95^19, 0.95^18, ..., 0.95^1, 0.95^0]

weighted_base_fee = Σ(block_fee[i] * weight[i]) / Σ(weight[i])

max_fee = weighted_base_fee * 1.2 + priority_fee
```

### Caching Strategy

- **Gas predictions:** 12 seconds (1 Ethereum block time)
- **Payment verifications:** No cache (always verify onchain)
- **Analytics:** 1 day retention in Redis

### Performance

- **Cached responses:** < 200ms
- **Uncached responses:** < 1s
- **Payment verification:** 2-5 seconds (depends on Base Sepolia)
- **Rate limit:** 10 requests/second per IP, burst 30

## Docker Deployment

### Build and Run

```bash
# Build image
docker-compose build

# Start services
docker-compose up -d

# View logs
docker-compose logs -f q-guard

# Stop services
docker-compose down
```

### Environment Variables in Docker

The `docker-compose.yml` automatically:
- Loads `.env` file
- Sets `REDIS_URL` to connect to Redis container
- Waits for Redis to be healthy before starting

## Development

### Project Structure

```
Q-guard/
├── src/
│   ├── main.rs           # Axum server entry point
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration loading
│   ├── error.rs          # Custom error types
│   ├── models/           # Data models
│   │   ├── gas.rs
│   │   ├── payment.rs
│   │   └── response.rs
│   ├── services/         # Business logic
│   │   ├── cache.rs      # Redis + moka cache
│   │   ├── ethereum.rs   # Gas prediction
│   │   ├── analytics.rs  # Payment tracking
│   │   └── reputation.rs # ERC-8004 (mocked)
│   ├── middleware/       # Request middleware
│   │   ├── x402.rs       # Payment verification
│   │   └── rate_limit.rs # Rate limiting
│   ├── handlers/         # HTTP handlers
│   │   ├── gas.rs
│   │   ├── health.rs
│   │   ├── stats.rs
│   │   └── dashboard.rs
│   └── client/           # Test client
│       ├── payment.rs    # USDC payment logic
│       └── test_agent.rs # CLI test tool
├── scripts/
│   ├── test_endpoints.sh
│   └── fund_testnet.sh
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
└── README.md
```

### Build Commands

```bash
# Check code
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Build release
cargo build --release
```

## Security

### Best Practices

- Private keys in environment variables only
- All Ethereum addresses validated
- Payment verification via onchain data
- Rate limiting per IP address
- CORS configured appropriately
- No sensitive data in logs

### Payment Security

1. **Onchain Verification:** Every payment is verified by checking the actual USDC transfer transaction on Base Sepolia
2. **Amount Validation:** Ensures payment amount meets or exceeds the required price
3. **Recipient Validation:** Confirms USDC was sent to the correct recipient address
4. **No Trusted Intermediaries:** Direct onchain verification, no reliance on external payment processors

## Troubleshooting

### "Insufficient USDC balance"

**Solution:**
- Get USDC from Base Sepolia faucet or bridge from Ethereum Sepolia
- Check balance: `cast balance <address> --rpc-url $BASE_SEPOLIA_RPC_URL`

### "Payment verification failed"

**Possible causes:**
- Transaction not confirmed yet (wait a few seconds)
- Wrong recipient address
- Insufficient USDC amount
- Transaction failed onchain

**Check transaction:**
```bash
# View on BaseScan
https://sepolia.basescan.org/tx/<tx_hash>
```

### "RPC error"

**Solution:**
- Verify RPC URL is correct
- Check API key hasn't exceeded rate limits
- Try fallback RPC if configured
- Use a different RPC provider (Alchemy, Infura, Ankr)

### "Redis connection failed"

**Impact:** API still works, uses in-memory cache only

**Solution:**
```bash
# Check Redis is running
docker-compose ps redis

# Restart Redis
docker-compose restart redis

# Check logs
docker-compose logs redis
```

## Monitoring

### WebSocket Dashboard

Connect to `ws://localhost:8080/ws/dashboard` for real-time metrics updated every second.

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## Support

- **GitHub Issues:** https://github.com/mjohnson518/Q-guard/issues
- **x402 Documentation:** https://x402.org
- **Base Sepolia Faucet:** https://www.coinbase.com/faucets/base-ethereum-goerli-faucet

---

**Built with Rust, Axum, and x402**

