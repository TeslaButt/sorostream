/** @type {import('next').NextConfig} */
const nextConfig = {
  // Required for Stellar SDK streaming / buffer compatibility in the browser
  webpack: (config) => {
    config.resolve.fallback = {
      ...config.resolve.fallback,
      buffer: require.resolve('buffer/'),
    };
    return config;
  },
};

module.exports = nextConfig;
