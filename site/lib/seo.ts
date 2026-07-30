import type { Metadata } from 'next';

export const siteName = 'Compactor';
export const siteUrl = 'https://compactor.greyharborsoftware.com';
export const siteDescription =
  'A lightweight, adapter-driven URL redirection service for deterministic redirects and privacy-aware request events.';
export const siteKeywords = [
  'URL redirection service',
  'redirect infrastructure',
  'Rust redirect service',
  'adapter-driven architecture',
  'privacy-aware request events',
  'JSON redirect configuration',
  'JSONL request events',
  'reverse proxy redirects',
] as const;

export const socialCard = {
  url: '/brand/social-card.png',
  width: 1200,
  height: 630,
  alt: 'Compactor — make the redirect, keep the machinery small',
} as const;

function withTrailingSlash(path: string): string {
  if (path === '/') {
    return path;
  }

  return path.endsWith('/') ? path : `${path}/`;
}

export function buildPageMetadata({
  title,
  description,
  canonicalPath,
}: {
  title: string;
  description: string | undefined;
  canonicalPath: string;
}): Metadata {
  const canonical = withTrailingSlash(canonicalPath);
  const resolvedDescription = description ?? siteDescription;

  return {
    title,
    description: resolvedDescription,
    alternates: {
      canonical,
    },
    openGraph: {
      title,
      description: resolvedDescription,
      url: canonical,
      siteName,
      type: 'website',
      images: [socialCard],
    },
    twitter: {
      card: 'summary_large_image',
      title,
      description: resolvedDescription,
      images: [socialCard.url],
    },
  };
}
