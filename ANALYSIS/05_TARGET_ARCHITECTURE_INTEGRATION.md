# 05: Target Integration Layer Architecture

**Phase 1 Day 3 - Task 3**  
**Status**: Design Complete  
**Scope**: REST/HTTP bridge protocol + contracts + versioning  
**Principles**: Zero ambiguity, backward compatible, security-first

---

## 1. Integration Architecture Overview

### Problem Statement
**Backend** (Rust consensus engine) + **Frontend** (C# MAUI client) = Need protocol bridge

**Solution**: REST/HTTP API with strict contracts, semantic versioning, typed DTOs

```
┌─────────────────────────┐
│   Frontend (MAUI)       │
│  - HttpClient           │
│  - RestApiClient        │
└────────┬────────────────┘
         │
         │ HTTP/REST (Protocol Layer)
         │ Request: JSON + Auth header
         │ Response: JSON + Status codes
         │
┌────────┴────────────────┐
│   API Gateway (Rust)    │
│  - axum/actix-web       │
│  - Protocol parsing     │
│  - Error mapping        │
└────────┬────────────────┘
         │
         │ Internal (no protocol)
         │
┌────────┴────────────────┐
│  Backend Services       │
│  - Consensus            │
│  - Identity             │
│  - Validation           │
└─────────────────────────┘
```

**Key Principle**: Protocol boundary is CRITICAL → Never mix protocol concerns with business logic

---

## 2. REST API Contract (Formal Definition)

### 2.1 Base Specification

```
Protocol: HTTPS (TLS 1.3+)
Host: api.neuroid.local (configurable)
Base URL: https://api.neuroid.local/api/v1

Encoding: UTF-8
Content-Type: application/json
Accept: application/json

Authentication: Bearer JWT (in Authorization header)
Rate Limit: 1000 requests/hour per identity

Versioning: Semantic (v1, v2, etc. in path)
Deprecation: X-API-Deprecated header + 6-month grace period
```

### 2.2 Standard Response Format

**All responses follow this structure**:

```json
{
  "status": "success|error|validation_error",
  "code": "HTTP_STATUS_CODE",
  "data": { /* entity or null */ },
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable message",
    "details": [ /* specific field errors */ ]
  },
  "meta": {
    "timestamp": "2025-12-19T10:46:32Z",
    "request_id": "req_abc123def456",
    "version": "1.0.0"
  }
}
```

**Example Success Response**:

```json
{
  "status": "success",
  "code": 200,
  "data": {
    "id": "tx_123",
    "status": "pending",
    "created_at": "2025-12-19T10:46:32Z"
  },
  "error": null,
  "meta": {
    "timestamp": "2025-12-19T10:46:33Z",
    "request_id": "req_abc123",
    "version": "1.0.0"
  }
}
```

**Example Error Response**:

```json
{
  "status": "validation_error",
  "code": 400,
  "data": null,
  "error": {
    "code": "INVALID_TRANSACTION",
    "message": "Transaction validation failed",
    "details": [
      {
        "field": "inputs",
        "message": "Transaction must have at least one input"
      },
      {
        "field": "outputs",
        "message": "Total outputs cannot exceed inputs"
      }
    ]
  },
  "meta": {
    "timestamp": "2025-12-19T10:46:33Z",
    "request_id": "req_abc123",
    "version": "1.0.0"
  }
}
```

### 2.3 HTTP Status Code Mapping

| Code | Meaning | When to Use |
|------|---------|------------|
| **200** | OK | Request succeeded |
| **201** | Created | Resource created (identity, tx) |
| **202** | Accepted | Async processing started (mining proposal) |
| **204** | No Content | Delete successful |
| **400** | Bad Request | Invalid input (missing fields) |
| **401** | Unauthorized | Missing or invalid JWT |
| **403** | Forbidden | Identity not authorized for resource |
| **404** | Not Found | Resource not found (identity, tx) |
| **409** | Conflict | Constraint violation (duplicate, fork) |
| **422** | Unprocessable Entity | Semantic validation failed (double-spend) |
| **429** | Too Many Requests | Rate limit exceeded |
| **500** | Internal Server Error | Backend error (never expose details) |
| **503** | Service Unavailable | Maintenance or temporary outage |

---

## 3. API Endpoint Definitions (Core)

### 3.1 Identity Endpoints

#### Register Identity
```
POST /api/v1/identity/register
Content-Type: application/json

Request:
{
  "username": "alice",
  "email": "alice@example.com",
  "public_key": "ed25519_base64_encoded_key"
}

Response (201):
{
  "id": "identity_abc123",
  "did": "did:neuro:abc123",
  "username": "alice",
  "public_key": "ed25519_...",
  "status": "active",
  "created_at": "2025-12-19T10:46:32Z"
}

Errors:
- 400: Invalid email format
- 400: Invalid public key format
- 409: Email already registered
- 422: Username reserved
```

#### Get Identity
```
GET /api/v1/identity/{did}
Authorization: Bearer {jwt_token}

Response (200):
{
  "id": "identity_abc123",
  "did": "did:neuro:abc123",
  "username": "alice",
  "public_key": "ed25519_...",
  "status": "active",
  "credentials": [
    {
      "id": "cred_123",
      "issuer": "did:neuro:issuer",
      "claims": { ... },
      "issued_at": "2025-12-19T10:46:32Z"
    }
  ],
  "created_at": "2025-12-19T10:46:32Z",
  "updated_at": "2025-12-19T10:46:32Z"
}

Errors:
- 401: Missing/invalid JWT
- 404: Identity not found
```

#### Verify Identity (Challenge-Response)
```
POST /api/v1/identity/{did}/verify
Authorization: Bearer {jwt_token}
Content-Type: application/json

Request:
{
  "challenge": "base64_encoded_random_bytes",
  "signature": "ed25519_signature_of_challenge"
}

Response (200):
{
  "verified": true,
  "proof": {
    "identity_id": "identity_abc123",
    "verified_at": "2025-12-19T10:46:32Z",
    "expires_at": "2025-12-19T11:46:32Z"
  }
}

Errors:
- 400: Invalid challenge format
- 400: Invalid signature format
- 422: Signature verification failed
```

#### Issue Credential
```
POST /api/v1/identity/{issuer_did}/credentials
Authorization: Bearer {issuer_jwt}
Content-Type: application/json

Request:
{
  "subject_did": "did:neuro:subject",
  "claims": {
    "name": "Alice",
    "email": "alice@example.com",
    "role": "validator"
  },
  "expires_in_days": 365
}

Response (201):
{
  "id": "cred_new123",
  "issuer": "did:neuro:issuer",
  "subject": "did:neuro:subject",
  "claims": { ... },
  "proof": {
    "algorithm": "JWS",
    "signature": "..."
  },
  "issued_at": "2025-12-19T10:46:32Z",
  "expires_at": "2026-12-19T10:46:32Z"
}

Errors:
- 401: Issuer not authorized
- 403: Issuer lacks credential issuance privilege
- 404: Subject identity not found
- 422: Claims validation failed
```

### 3.2 Transaction Endpoints

#### Submit Transaction
```
POST /api/v1/transactions
Authorization: Bearer {jwt_token}
Content-Type: application/json

Request:
{
  "inputs": [
    {
      "previous_tx_id": "tx_prev123",
      "output_index": 0,
      "signature": "ed25519_signature"
    }
  ],
  "outputs": [
    {
      "amount": "1000000000",  // Satoshis
      "recipient": "did:neuro:recipient"
    }
  ],
  "fee": "10000",
  "signature": "ed25519_signature_of_tx"
}

Response (202):
{
  "id": "tx_new123",
  "status": "pending",
  "hash": "sha256_hash",
  "created_at": "2025-12-19T10:46:32Z",
  "fee": "10000"
}

Errors:
- 400: Invalid input format
- 400: Signature verification failed
- 403: Sender not authenticated
- 422: Double-spend detected
- 422: Insufficient balance
- 422: Fee too low
```

#### Get Transaction
```
GET /api/v1/transactions/{tx_id}
Authorization: Bearer {jwt_token}

Response (200):
{
  "id": "tx_123",
  "hash": "sha256_...",
  "status": "confirmed",  // pending, confirmed, failed
  "inputs": [ ... ],
  "outputs": [ ... ],
  "block_height": 12345,
  "confirmations": 10,
  "created_at": "2025-12-19T10:46:32Z",
  "confirmed_at": "2025-12-19T10:47:00Z"
}

Errors:
- 401: Missing/invalid JWT
- 404: Transaction not found
```

#### List Pending Transactions
```
GET /api/v1/transactions?status=pending&limit=50&offset=0
Authorization: Bearer {jwt_token}

Response (200):
{
  "transactions": [
    { /* transaction objects */ }
  ],
  "total": 1234,
  "limit": 50,
  "offset": 0
}

Errors:
- 401: Missing/invalid JWT
- 400: Invalid pagination parameters
```

### 3.3 Consensus Endpoints

#### Get Chain State
```
GET /api/v1/consensus/chain-state
(No auth required - public data)

Response (200):
{
  "height": 12345,
  "tip_hash": "sha256_...",
  "tip_timestamp": "2025-12-19T10:46:32Z",
  "median_block_time_ms": 6000,
  "total_transactions": 987654,
  "total_output_value": "21000000000000000",  // Satoshis
  "network_difficulty": 256.5,
  "consensus_algorithm": "dag_with_proof_of_work"
}
```

#### Get Block
```
GET /api/v1/consensus/blocks/{block_id_or_height}

Response (200):
{
  "id": "block_123",
  "height": 12345,
  "hash": "sha256_...",
  "parent_hash": "sha256_...",
  "timestamp": "2025-12-19T10:46:32Z",
  "miner": "did:neuro:miner",
  "difficulty": 256,
  "nonce": 123456789,
  "transactions": [ /* tx hashes */ ],
  "merkle_root": "sha256_...",
  "confirmations": 10
}

Errors:
- 404: Block not found
```

---

## 4. Authentication & Security

### 4.1 JWT Token Structure

```json
{
  "header": {
    "alg": "Ed25519",
    "typ": "JWT",
    "kid": "key_id_abc123"
  },
  "payload": {
    "iss": "did:neuro:issuer",              // Backend identity
    "sub": "did:neuro:user",                // User identity
    "aud": "api.neuroid.local",
    "exp": 1703001600,                      // Unix timestamp
    "iat": 1702998000,
    "jti": "jwt_abc123def456",              // Unique ID
    "scope": ["identity:read", "transaction:write"],
    "identity_verified": true,
    "biometric_authenticated": false
  },
  "signature": "ed25519_signature"
}
```

### 4.2 Token Lifecycle

```
1. User authenticates on frontend
2. Frontend generates challenge (random bytes)
3. User signs challenge with private key
4. Frontend sends (identity_did, signature) to backend
5. Backend verifies signature against public key
6. Backend generates JWT token (1-hour expiry)
7. Frontend stores JWT in secure storage (Keychain/Keystore)
8. Frontend includes JWT in every request (Authorization header)
9. Backend validates JWT on every request
10. Token refresh: 15 minutes before expiry, get new token
```

### 4.3 Request Signing (DTO Security)

**All sensitive requests must be signed**:

```rust
// Backend receives TransactionRequest
pub struct TransactionRequest {
    pub inputs: Vec<InputRequest>,
    pub outputs: Vec<OutputRequest>,
    pub signature: String,  // Ed25519 signature of request body
    pub nonce: u64,         // Prevent replay attacks
    pub timestamp: u64,     // Request timestamp (reject if >5min old)
}

// Verification logic:
// 1. Parse request body → canonical JSON
// 2. Hash with SHA-256
// 3. Verify Ed25519 signature with sender's public key
// 4. Check nonce not seen before (prevent replay)
// 5. Check timestamp within 5-minute window
```

---

## 5. Semantic Versioning & Backward Compatibility

### 5.1 Version Strategy

```
API Version: MAJOR.MINOR.PATCH
Example: v1.2.3

MAJOR: Breaking changes (URL path: /api/v1 → /api/v2)
MINOR: New features (backward compatible)
PATCH: Bug fixes (backward compatible)

URL Pattern: /api/v{MAJOR}/resource
Old versions: Supported for 12 months, then deprecated for 6 months, then removed
```

### 5.2 Deprecation Policy

**When to deprecate**:
```
1. Announce 12 months in advance
2. Add X-API-Deprecated: true header
3. Add X-API-Sunset: <RFC 7231 date> header
4. Log warning in response body
5. After 12 months: Support only 6 more months
6. After 18 months: Remove endpoint entirely
```

**Example deprecation response**:
```json
{
  "status": "success",
  "code": 200,
  "data": { /* response */ },
  "meta": {
    "x-api-deprecated": true,
    "x-api-sunset": "2026-12-19T00:00:00Z",
    "x-api-deprecation-warning": "This endpoint will be removed on 2026-12-19. Please migrate to POST /api/v2/transactions"
  }
}
```

### 5.3 Backward Compatibility Rules

**Never break these**:
1. Existing request fields are required → Can add optional fields
2. Existing response fields are required → Can add optional fields
3. HTTP status codes for success (2xx) are fixed
4. Error structure is fixed (keep error.code, error.message, error.details)

**Can change**:
1. Add new optional request fields
2. Add new optional response fields
3. Add new error codes (extend enum)
4. Change internal implementation (never visible to client)

---

## 6. Error Handling Specification

### 6.1 Error Code Catalog

```
// Identity Errors
IDENTITY_NOT_FOUND = "IDENTITY_NOT_FOUND"
IDENTITY_ALREADY_EXISTS = "IDENTITY_ALREADY_EXISTS"
IDENTITY_REVOKED = "IDENTITY_REVOKED"
IDENTITY_SUSPENDED = "IDENTITY_SUSPENDED"

// Credential Errors
CREDENTIAL_INVALID_SIGNATURE = "CREDENTIAL_INVALID_SIGNATURE"
CREDENTIAL_EXPIRED = "CREDENTIAL_EXPIRED"
CREDENTIAL_NOT_FOUND = "CREDENTIAL_NOT_FOUND"
CREDENTIAL_UNAUTHORIZED_ISSUER = "CREDENTIAL_UNAUTHORIZED_ISSUER"

// Transaction Errors
TRANSACTION_NOT_FOUND = "TRANSACTION_NOT_FOUND"
TRANSACTION_DOUBLE_SPEND = "TRANSACTION_DOUBLE_SPEND"
TRANSACTION_INVALID_INPUT = "TRANSACTION_INVALID_INPUT"
TRANSACTION_INVALID_OUTPUT = "TRANSACTION_INVALID_OUTPUT"
TRANSACTION_INSUFFICIENT_BALANCE = "TRANSACTION_INSUFFICIENT_BALANCE"
TRANSACTION_FEE_TOO_LOW = "TRANSACTION_FEE_TOO_LOW"
TRANSACTION_SIGNATURE_INVALID = "TRANSACTION_SIGNATURE_INVALID"

// Consensus Errors
CONSENSUS_FORK_DETECTED = "CONSENSUS_FORK_DETECTED"
CONSENSUS_BLOCK_INVALID = "CONSENSUS_BLOCK_INVALID"
CONSENSUS_DIFFICULTY_ADJUSTMENT = "CONSENSUS_DIFFICULTY_ADJUSTMENT"

// Auth Errors
AUTH_MISSING_TOKEN = "AUTH_MISSING_TOKEN"
AUTH_INVALID_TOKEN = "AUTH_INVALID_TOKEN"
AUTH_TOKEN_EXPIRED = "AUTH_TOKEN_EXPIRED"
AUTH_UNAUTHORIZED = "AUTH_UNAUTHORIZED"
AUTH_INVALID_SIGNATURE = "AUTH_INVALID_SIGNATURE"

// Validation Errors
VALIDATION_INVALID_EMAIL = "VALIDATION_INVALID_EMAIL"
VALIDATION_INVALID_KEY_FORMAT = "VALIDATION_INVALID_KEY_FORMAT"
VALIDATION_INVALID_SIGNATURE_FORMAT = "VALIDATION_INVALID_SIGNATURE_FORMAT"
VALIDATION_MISSING_REQUIRED_FIELD = "VALIDATION_MISSING_REQUIRED_FIELD"

// Rate Limit
RATE_LIMIT_EXCEEDED = "RATE_LIMIT_EXCEEDED"

// Server Errors
SERVER_ERROR = "SERVER_ERROR"
SERVICE_UNAVAILABLE = "SERVICE_UNAVAILABLE"
```

### 6.2 Error Response Examples

**Validation Error** (400):
```json
{
  "status": "validation_error",
  "code": 400,
  "error": {
    "code": "VALIDATION_MISSING_REQUIRED_FIELD",
    "message": "Required fields missing",
    "details": [
      { "field": "inputs", "message": "inputs is required" },
      { "field": "outputs", "message": "outputs is required" }
    ]
  }
}
```

**Authentication Error** (401):
```json
{
  "status": "error",
  "code": 401,
  "error": {
    "code": "AUTH_INVALID_TOKEN",
    "message": "Invalid or expired token",
    "details": []
  }
}
```

**Business Logic Error** (422):
```json
{
  "status": "error",
  "code": 422,
  "error": {
    "code": "TRANSACTION_DOUBLE_SPEND",
    "message": "Transaction references already-spent outputs",
    "details": [
      {
        "field": "inputs[0]",
        "message": "Output already spent in block 12340"
      }
    ]
  }
}
```

---

## 7. Protocol Implementation (Frontend Client)

### 7.1 HTTP Client Setup

```csharp
// File: Features/Services/Api/HttpClientConfiguration.cs

public static class HttpClientConfiguration
{
    public static IHttpClientBuilder AddRestApiClient(this IServiceCollection services, string apiBaseUrl)
    {
        return services
            .AddHttpClient<IApiClient, RestApiClient>(client =>
            {
                client.BaseAddress = new Uri(apiBaseUrl);
                client.DefaultRequestHeaders.Add("Accept", "application/json");
                client.DefaultRequestHeaders.Add("User-Agent", "NeuroID-Client/1.0");
                client.Timeout = TimeSpan.FromSeconds(30);
            })
            .AddPolicyHandler(GetRetryPolicy())
            .AddPolicyHandler(GetCircuitBreakerPolicy());
    }
    
    private static IAsyncPolicy<HttpResponseMessage> GetRetryPolicy()
    {
        return HttpPolicyExtensions
            .HandleTransientHttpError()
            .OrResult(r => r.StatusCode == System.Net.HttpStatusCode.TooManyRequests)
            .WaitAndRetryAsync(
                retryCount: 3,
                sleepDurationProvider: attempt => TimeSpan.FromMilliseconds(Math.Pow(2, attempt) * 100)
            );
    }
    
    private static IAsyncPolicy<HttpResponseMessage> GetCircuitBreakerPolicy()
    {
        return HttpPolicyExtensions
            .HandleTransientHttpError()
            .CircuitBreakerAsync(
                handledEventsAllowedBeforeBreaking: 5,
                durationOfBreak: TimeSpan.FromSeconds(30)
            );
    }
}
```

### 7.2 Request/Response Handling

```csharp
// File: Features/Services/Api/RestApiClient.cs

public class RestApiClient : IApiClient
{
    private readonly HttpClient _httpClient;
    private readonly IJwtTokenService _tokenService;
    private readonly ILogger<RestApiClient> _logger;
    
    public async Task<TransactionResponse> SubmitTransactionAsync(TransactionRequest request)
    {
        try
        {
            // Add JWT to header
            var jwt = await _tokenService.GetValidTokenAsync();
            _httpClient.DefaultRequestHeaders.Authorization = 
                new AuthenticationHeaderValue("Bearer", jwt);
            
            // Serialize request
            var json = JsonSerializer.Serialize(request, JsonOptions);
            var content = new StringContent(json, Encoding.UTF8, "application/json");
            
            // Send request with retry
            var response = await _httpClient.PostAsync("/api/v1/transactions", content);
            
            // Parse response
            var responseJson = await response.Content.ReadAsStringAsync();
            var apiResponse = JsonSerializer.Deserialize<ApiResponse<TransactionResponse>>(responseJson);
            
            // Handle response status
            if (!response.IsSuccessStatusCode)
            {
                return HandleErrorResponse(response.StatusCode, apiResponse?.Error);
            }
            
            return apiResponse.Data;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError($"Network error: {ex.Message}");
            throw new ServiceException("Network error", ex);
        }
    }
    
    private T HandleErrorResponse<T>(System.Net.HttpStatusCode statusCode, ErrorResponse error)
    {
        var errorCode = error?.Code ?? "SERVER_ERROR";
        var message = error?.Message ?? "Unknown error";
        
        _logger.LogError($"API Error: {errorCode} - {message}");
        
        throw statusCode switch
        {
            System.Net.HttpStatusCode.BadRequest => 
                new ValidationException(message, error?.Details),
            System.Net.HttpStatusCode.Unauthorized => 
                new AuthenticationException(message),
            System.Net.HttpStatusCode.Forbidden => 
                new AuthorizationException(message),
            System.Net.HttpStatusCode.NotFound => 
                new NotFoundException(message),
            System.Net.HttpStatusCode.Conflict => 
                new ConflictException(message, errorCode),
            System.Net.HttpStatusCode.UnprocessableEntity => 
                new DomainException(message, errorCode),
            System.Net.HttpStatusCode.TooManyRequests => 
                new RateLimitException(message),
            _ => new ServiceException(message, error?.Code)
        };
    }
}
```

---

## 8. Protocol Testing Strategy

### 8.1 Contract Tests

```csharp
[TestClass]
public class ApiContractTests
{
    private RestApiClient _client;
    private MockHttpMessageHandler _mockHttp;
    
    [TestInitialize]
    public void Setup()
    {
        _mockHttp = new MockHttpMessageHandler();
        var httpClient = new HttpClient(_mockHttp) { BaseAddress = new Uri("https://api.test") };
        _client = new RestApiClient(httpClient, null);
    }
    
    [TestMethod]
    public async Task SubmitTransaction_Success_ReturnsValidResponse()
    {
        // Arrange
        var expectedResponse = new ApiResponse<TransactionResponse>
        {
            Status = "success",
            Code = 202,
            Data = new TransactionResponse { Id = "tx_123", Status = "pending" }
        };
        
        _mockHttp.When(HttpMethod.Post, "https://api.test/api/v1/transactions")
            .Respond(System.Net.HttpStatusCode.Accepted, "application/json", 
                JsonSerializer.Serialize(expectedResponse));
        
        // Act
        var result = await _client.SubmitTransactionAsync(new TransactionRequest());
        
        // Assert
        Assert.AreEqual("tx_123", result.Id);
        Assert.AreEqual("pending", result.Status);
    }
    
    [TestMethod]
    public async Task SubmitTransaction_ValidationError_ThrowsValidationException()
    {
        // Arrange
        var errorResponse = new ApiResponse<object>
        {
            Status = "validation_error",
            Code = 400,
            Error = new ErrorResponse
            {
                Code = "VALIDATION_MISSING_REQUIRED_FIELD",
                Message = "Missing required fields",
                Details = new[] { new ErrorDetail { Field = "inputs", Message = "inputs is required" } }
            }
        };
        
        _mockHttp.When(HttpMethod.Post, "https://api.test/api/v1/transactions")
            .Respond(System.Net.HttpStatusCode.BadRequest, "application/json",
                JsonSerializer.Serialize(errorResponse));
        
        // Act & Assert
        await Assert.ThrowsExceptionAsync<ValidationException>(
            () => _client.SubmitTransactionAsync(new TransactionRequest())
        );
    }
}
```

---

## 9. Monitoring & Observability

### 9.1 Request/Response Logging

```csharp
public class ApiLoggingHandler : DelegatingHandler
{
    private readonly ILogger<ApiLoggingHandler> _logger;
    
    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request, 
        CancellationToken cancellationToken)
    {
        var requestId = request.Headers.GetValues("X-Request-ID").FirstOrDefault() 
            ?? Guid.NewGuid().ToString();
        
        _logger.LogInformation(
            "API Request: {method} {url} [RequestId: {requestId}]",
            request.Method, request.RequestUri, requestId
        );
        
        var sw = Stopwatch.StartNew();
        var response = await base.SendAsync(request, cancellationToken);
        sw.Stop();
        
        _logger.LogInformation(
            "API Response: {statusCode} {reason} {elapsedMs}ms [RequestId: {requestId}]",
            (int)response.StatusCode, response.ReasonPhrase, sw.ElapsedMilliseconds, requestId
        );
        
        return response;
    }
}
```

---

## 10. Summary: Protocol Contract

| Aspect | Specification | Purpose |
|--------|---|---|
| **Protocol** | HTTPS/TLS 1.3+ | Security |
| **Format** | JSON with strict schema | Interoperability |
| **Versioning** | Semantic v1, v2, etc. | Backward compatibility |
| **Auth** | JWT + Ed25519 signing | Security + Non-repudiation |
| **Errors** | Typed error codes + details | Debuggability |
| **Status Codes** | HTTP standard 2xx/4xx/5xx | REST compliance |
| **Rate Limit** | 1000 req/hour per identity | Abuse prevention |
| **Timeout** | 30 seconds | Network stability |

**Principle**: Protocol is contract → Breaking changes = major version bump → No ambiguity

---

**End of Integration Layer Architecture**

*Next: Task 4 - Compliance & Security Design (EU GDPR + eIDAS)*
