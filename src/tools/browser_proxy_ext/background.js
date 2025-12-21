// Proxy configuration - values are replaced at runtime
const PROXY_HOST = "{{PROXY_HOST}}";
const PROXY_PORT = {{PROXY_PORT}};
const PROXY_USER = "{{PROXY_USER}}";
const PROXY_PASS = "{{PROXY_PASS}}";
const PROXY_SCHEME = "{{PROXY_SCHEME}}"; // http, https, or socks5

// Configure proxy settings
const proxyConfig = {
  mode: "fixed_servers",
  rules: {
    singleProxy: {
      scheme: PROXY_SCHEME === "socks5" ? "socks5" : "http",
      host: PROXY_HOST,
      port: PROXY_PORT
    },
    bypassList: ["localhost", "127.0.0.1"]
  }
};

// Set proxy configuration
chrome.proxy.settings.set(
  { value: proxyConfig, scope: "regular" },
  () => {
    console.log("Proxy configured:", PROXY_HOST + ":" + PROXY_PORT);
  }
);

// Handle proxy authentication (only works for HTTP/HTTPS proxies)
chrome.webRequest.onAuthRequired.addListener(
  (details, callback) => {
    console.log("Auth required for:", details.challenger);
    callback({
      authCredentials: {
        username: PROXY_USER,
        password: PROXY_PASS
      }
    });
  },
  { urls: ["<all_urls>"] },
  ["asyncBlocking"]
);

console.log("Proxy extension loaded - scheme:", PROXY_SCHEME, "host:", PROXY_HOST, "port:", PROXY_PORT);
