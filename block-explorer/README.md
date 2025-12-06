# Rust Blockchain Explorer

A modern web-based block explorer for the Rust Blockchain network.

## Features

- 📊 Real-time blockchain statistics
- 🔍 Browse blocks and transactions
- 💼 View wallet balances and transactions
- 🔗 Navigate blockchain links
- 📱 Responsive design

## Getting Started

### Prerequisites

- Node.js 18+ and npm/yarn
- Rust Blockchain server running on `http://127.0.0.1:8080`

### Installation

```bash
# Install dependencies
npm install

# Run development server
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

### Build for Production

```bash
# Build the application
npm run build

# Start production server
npm start
```

## Configuration

Set the API URL via environment variable:

```bash
API_URL=http://127.0.0.1:8080/api/v1 npm run dev
```

Or create a `.env.local` file:

```
API_URL=http://127.0.0.1:8080/api/v1
```

## Features

### Home Page
- Overview statistics (blocks, transactions, peers)
- Latest blocks table
- Search functionality

### Block Page
- Detailed block information
- List of all transactions in the block
- Navigation to previous blocks

## Technology Stack

- **Next.js 14** - React framework
- **TypeScript** - Type safety
- **Tailwind CSS** - Styling
- **Axios** - HTTP client

## License

MIT

