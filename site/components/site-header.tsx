import Link from 'next/link';

const navItems = [
  { href: '/docs', label: 'Docs' },
  { href: '/docs/tutorials', label: 'Tutorial' },
  { href: '/docs/how-to', label: 'How-to' },
  { href: '/docs/reference', label: 'Reference' },
  { href: '/docs/explanation', label: 'Explanation' },
] as const;

export function SiteHeader() {
  return (
    <header className="topbar">
      <Link className="brand" href="/" aria-label="Compactor home">
        <span className="brand-mark" aria-hidden="true">
          <img src="/brand/compactor-mark.svg" alt="" width={54} height={54} />
        </span>
        <span className="brand-copy">
          <span className="brand-name">Compactor</span>
          <span className="brand-tag">Redirects without the machinery</span>
        </span>
      </Link>

      <div className="topbar-actions">
        <nav className="topnav" aria-label="Primary">
          {navItems.map((item) => (
            <Link key={item.href} href={item.href}>
              {item.label}
            </Link>
          ))}
        </nav>

        <a
          className="repo-link"
          href="https://github.com/Grey-Harbor/Compactor"
          aria-label="Compactor on GitHub"
          target="_blank"
          rel="noreferrer"
        >
          GitHub
        </a>
      </div>
    </header>
  );
}
