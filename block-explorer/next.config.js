/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  env: {
    API_URL: process.env.API_URL || 'http://127.0.0.1:8080/api/v1',
  },
}

module.exports = nextConfig

