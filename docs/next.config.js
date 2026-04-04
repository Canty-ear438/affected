/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  images: { unoptimized: true },
  basePath: process.env.NODE_ENV === "production" ? "/affected" : "",
  assetPrefix: process.env.NODE_ENV === "production" ? "/affected" : "",
};

module.exports = nextConfig;
