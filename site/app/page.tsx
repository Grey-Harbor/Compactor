import type { Metadata } from 'next';
import Link from 'next/link';

import { SiteFooter } from '@/components/site-footer';
import { buildPageMetadata, siteDescription, siteName, siteUrl, socialCard } from '@/lib/seo';

export const metadata: Metadata = buildPageMetadata({
  title: 'Small, deterministic redirect infrastructure',
  description: siteDescription,
  canonicalPath: '/',
});

const principles = [
  {
    title: 'Deterministic redirects',
    description:
      'Normalize each public URL once, resolve an exact definition, and return a deliberate redirect response.',
  },
  {
    title: 'Adapters stay independent',
    description:
      'Redirect definitions and request events cross separate ports, keeping configuration and analysis replaceable.',
  },
  {
    title: 'Privacy has bounds',
    description:
      'Capture only allowlisted request metadata, truncate it predictably, and keep credentials out of events.',
  },
] as const;

const paths = [
  {
    title: 'Tutorial',
    description: 'Run Compactor locally and make your first redirect.',
    href: '/docs/tutorials',
  },
  {
    title: 'How-to',
    description: 'Deploy, configure, operate, and publish Compactor.',
    href: '/docs/how-to',
  },
  {
    title: 'Explanation',
    description: 'Understand the adapter model and deliberately narrow product boundary.',
    href: '/docs/explanation',
  },
  {
    title: 'Reference',
    description: 'Look up configuration, source, event, and URL contracts.',
    href: '/docs/reference',
  },
] as const;

const useCases = [
  {
    title: 'Retired routes',
    description: 'Move durable public URLs without carrying an application framework into the redirect tier.',
  },
  {
    title: 'Service migrations',
    description:
      'Route old hosts and paths to their new homes with exact, reviewable definitions.',
  },
  {
    title: 'Campaign handoffs',
    description:
      'Forward stable entry points while leaving campaign management and analysis in their own systems.',
  },
] as const;

const structuredData = {
  '@context': 'https://schema.org',
  '@type': 'SoftwareApplication',
  name: siteName,
  applicationCategory: 'DeveloperApplication',
  operatingSystem: 'Any',
  description: siteDescription,
  url: siteUrl,
  image: new URL(socialCard.url, siteUrl).toString(),
  license: 'https://www.apache.org/licenses/LICENSE-2.0',
};

export default function HomePage() {
  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(structuredData) }}
      />
      <main className="landing" id="main">
        <section className="hero">
          <div className="hero-copy">
            <span className="eyebrow">Small redirect infrastructure</span>
            <h1>Compactor</h1>
            <p className="lede">Make the redirect. Keep the machinery small.</p>
            <p className="hero-detail">
              Compactor resolves configured public URLs, returns precise redirects, and emits
              bounded request events—without becoming a link-management product.
            </p>
            <div className="actions">
              <Link className="button primary" href="/docs/tutorials/getting-started">
                Start with the tutorial
              </Link>
              <Link className="button secondary" href="/docs/reference/configuration">
                See the configuration
              </Link>
            </div>
          </div>

          <aside className="hero-panel" aria-label="Compactor request flow">
            <div className="flow-label">request</div>
            <div className="flow-track" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
            <div className="flow-step">
              <strong>Canonical public URL</strong>
              <p>Preserve meaningful path identity and leave queries out of lookup.</p>
            </div>
            <div className="flow-step">
              <strong>One exact definition</strong>
              <p>Resolve through the configured source adapter and construct the response.</p>
            </div>
            <div className="flow-result">redirect + event</div>
          </aside>
        </section>

        <section className="section" aria-labelledby="principles-heading">
          <div className="section-heading">
            <p className="eyebrow">Focused by design</p>
            <h2 id="principles-heading">One request-time responsibility</h2>
            <p>
              Compactor owns redirect resolution and a bounded account of the completed request.
              Everything else stays with the systems built for it.
            </p>
          </div>
          <div className="card-grid">
            {principles.map((principle) => (
              <article className="info-card" key={principle.title}>
                <h3>{principle.title}</h3>
                <p>{principle.description}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="section why-section" aria-labelledby="why-heading">
          <div className="section-heading">
            <p className="eyebrow">Why it exists</p>
            <h2 id="why-heading">Infrastructure, not a link-management product</h2>
            <p>
              No dashboard, campaigns, accounts, mutation API, or analytics engine. Redirect
              definitions come from configuration; events go to an external collector. Compactor
              keeps the boundary visible and the runtime ordinary.
            </p>
          </div>
        </section>

        <section className="section" aria-labelledby="docs-heading">
          <div className="section-heading">
            <p className="eyebrow">Guides &amp; reference</p>
            <h2 id="docs-heading">Choose your path</h2>
            <p>Learn, do, understand, or look up without mixing those jobs together.</p>
          </div>
          <div className="path-grid">
            {paths.map((path) => (
              <article className="path-card" key={path.title}>
                <h3>{path.title}</h3>
                <p>{path.description}</p>
                <Link href={path.href}>Open {path.title.toLowerCase()}</Link>
              </article>
            ))}
          </div>
        </section>

        <section className="section" aria-labelledby="fits-heading">
          <div className="section-heading">
            <p className="eyebrow">Where it fits</p>
            <h2 id="fits-heading">Stable URLs at the edge of change</h2>
          </div>
          <div className="story-grid">
            {useCases.map((useCase) => (
              <article className="story-card" key={useCase.title}>
                <h3>{useCase.title}</h3>
                <p>{useCase.description}</p>
              </article>
            ))}
          </div>
        </section>

        <SiteFooter />
      </main>
    </>
  );
}
