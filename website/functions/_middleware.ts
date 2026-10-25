/**
 * Cloudflare Pages Edge Middleware for portctl.githsoft.com
 * Handles dual-mode content negotiation:
 * - curl / wget / httpie -> 302 redirect to /install.sh
 * - PowerShell on root -> 302 redirect to /install.ps1
 * - Browser / standard HTTP client -> serves HTML documentation
 */

interface Env {
  ASSETS: {
    fetch: typeof fetch;
  };
}

export const onRequest: PagesFunction<Env> = async (context) => {
  const { request, next } = context;
  const url = new URL(request.url);
  const userAgent = (request.headers.get('user-agent') || '').toLowerCase();
  const accept = (request.headers.get('accept') || '').toLowerCase();

  // Handle Root Path ("/")
  if (url.pathname === '/' || url.pathname === '') {
    const isCurlOrWget = /curl|wget|httpie|fetch/i.test(userAgent);
    const isPowerShell = /powershell|winhttp/i.test(userAgent);

    // If client is a terminal tool without HTML preference
    if (isCurlOrWget && !accept.includes('text/html')) {
      return Response.redirect(`${url.origin}/install.sh`, 302);
    }

    if (isPowerShell && !accept.includes('text/html')) {
      return Response.redirect(`${url.origin}/install.ps1`, 302);
    }
  }

  // Proceed to static assets or page handlers
  const response = await next();

  // Clone headers to inject Vary: User-Agent, Accept
  // Crucial: Prevents CDN from serving shell script to browsers or HTML to curl
  const newHeaders = new Headers(response.headers);
  newHeaders.set('Vary', 'User-Agent, Accept');
  newHeaders.set('X-Content-Type-Options', 'nosniff');

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: newHeaders,
  });
};
