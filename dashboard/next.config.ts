import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  ...(process.env.STANDALONE === "true" ? { output: "standalone" as const } : {}),
  turbopack: {
    root: process.cwd(),
  },
};

export default nextConfig;
