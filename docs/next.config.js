/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  images: { unoptimized: true },
  basePath: "/affected",
  assetPrefix: "/affected",
};

module.exports = nextConfig;
