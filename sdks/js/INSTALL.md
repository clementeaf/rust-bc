# Installation Guide

## Prerequisites

- Node.js 16+ and npm/yarn
- TypeScript 5.0+ (for development)

## Installation

```bash
# Install dependencies
npm install

# Build the SDK
npm run build

# Run examples (after building)
npx ts-node examples/basic-usage.ts
npx ts-node examples/transactions.ts
npx ts-node examples/billing.ts
```

## Development

```bash
# Watch mode for development
npm run dev

# Run tests
npm test
```

## Publishing

```bash
# Build before publishing
npm run build

# Publish to npm (if you have access)
npm publish
```

