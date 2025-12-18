// Auto-reload when page is updated (for watch mode)
const pageLoadTime = Date.now();
setInterval(async () => {
  try {
    const response = await fetch(window.location.href, {
      method: "HEAD",
      cache: "no-cache",
    });
    const lastModified = response.headers.get("Last-Modified");
    if (lastModified) {
      const modifiedTime = new Date(lastModified).getTime();
      if (modifiedTime > pageLoadTime) {
        window.location.reload();
      }
    }
  } catch (e) {
    // Ignore errors (e.g., if server is temporarily down)
  }
}, 1000);
