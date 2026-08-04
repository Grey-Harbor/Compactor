import Link from 'next/link';

export function SiteFooter() {
  return (
    <footer className="site-footer">
      <div className="footer-links" aria-label="Related links">
        <Link href="/docs/how-to/deploy-with-docker">Deploy with Docker</Link>
        <a href="https://github.com/Grey-Harbor/Compactor" target="_blank" rel="noreferrer">
          GitHub repository
        </a>
        <a href="https://www.greyharborsoftware.com" target="_blank" rel="noreferrer">
          Grey Harbor Software
        </a>
      </div>
      <p>&copy; {new Date().getFullYear()} Grey Harbor Software. Apache-2.0 licensed.</p>
    </footer>
  );
}
