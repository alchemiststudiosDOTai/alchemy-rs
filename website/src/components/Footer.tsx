import { Link } from "react-router-dom";
import { Logo } from "./Logo";

type FooterLink = {
  label: string;
  href: string;
  external?: boolean;
};

const FOOTER_LINKS: Record<string, FooterLink[]> = {
  Documentation: [
    { label: "Getting Started", href: "/docs" },
    { label: "Providers", href: "/docs/providers" },
    { label: "Tool Calling", href: "/docs/tools" },
    { label: "Streaming", href: "/docs/streaming" },
  ],
  Community: [
    { label: "GitHub", href: "https://github.com/alchemiststudiosDOTai/alchemy-rs", external: true },
    { label: "Discord", href: "https://discord.gg/alchemy", external: true },
    { label: "Twitter", href: "https://twitter.com/alchemy_rs", external: true },
  ],
  Resources: [
    { label: "Examples", href: "/docs/examples" },
    { label: "API Reference", href: "/docs/api" },
    { label: "Changelog", href: "https://github.com/alchemiststudiosDOTai/alchemy-rs/releases", external: true },
  ],
};

export function Footer() {
  return (
    <footer className="container mx-auto border-x border-b">
      {/* Main footer content */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_2fr]">
        {/* Left: Logo and description */}
        <div className="p-6 lg:p-12 border-b lg:border-b-0 lg:border-r">
          <Logo width={80} height={46} className="mb-4" />
          <p className="text-muted-foreground text-sm max-w-[240px]">
            Unified LLM API abstraction layer in Rust.
          </p>
        </div>

        {/* Right: Link columns */}
        <div className="grid grid-cols-2 md:grid-cols-3">
          {Object.entries(FOOTER_LINKS).map(([category, links], index) => (
            <div
              key={category}
              className={`p-6 lg:p-12 border-b md:border-b-0 last:border-b-0 ${
                index < Object.keys(FOOTER_LINKS).length - 1 ? "border-r" : ""
              }`}
            >
              <div className="font-medium mb-4">{category}</div>
              <ul className="space-y-3">
                {links.map((link) => (
                  <li key={link.label}>
                    {link.external ? (
                      <a
                        href={link.href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-muted-foreground text-sm hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </a>
                    ) : (
                      <Link
                        to={link.href}
                        className="text-muted-foreground text-sm hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </Link>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </div>

      {/* Bottom bar */}
      <div className="border-t p-6 lg:p-12 flex flex-col md:flex-row justify-between items-center gap-4 text-sm text-muted-foreground">
        <div>© {new Date().getFullYear()} Alchemy. MIT License.</div>
        <div className="font-mono text-xs">cargo add alchemy</div>
      </div>
    </footer>
  );
}
