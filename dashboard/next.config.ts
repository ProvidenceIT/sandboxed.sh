import type { NextConfig } from "next";
import { readFileSync } from "fs";

const { version } = JSON.parse(readFileSync("./package.json", "utf-8"));

const nextConfig: NextConfig = {
  ...(process.env.STANDALONE === "true" ? { output: "standalone" as const } : {}),
  env: {
    NEXT_PUBLIC_APP_VERSION: version,
  },
  turbopack: {
    root: process.cwd(),
  },
};

export default nextConfig;
