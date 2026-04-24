// User-agent header parsing — has nothing to do with AI agents.
export function parseUserAgent(header: string): { browser: string; os: string } {
  const ua = header.toLowerCase();
  const browser = ua.includes("firefox") ? "firefox" : "other";
  const os = ua.includes("mac") ? "macos" : "other";
  return { browser, os };
}
