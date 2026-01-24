import { Link } from "react-router-dom";

export function CTASection() {
  return (
    <section className="relative border-y mt-12">
      {/* Content */}
      <div className="relative z-10 p-8 py-12 lg:p-24 text-center">
        <h2 className="text-3xl lg:text-5xl mb-2 lg:mb-4">
          Ready to build with Alchemy?
        </h2>
        <p className="text-muted-foreground lg:text-xl mb-6 lg:mb-12 max-w-2xl mx-auto">
          Get started in minutes. Install via Cargo and start building AI-powered
          applications with a unified, type-safe API.
        </p>
        <div className="flex flex-col sm:flex-row gap-4 lg:gap-6 justify-center mb-6 lg:mb-12">
          <Link
            to="/docs"
            className="inline-flex items-center justify-center px-6 py-3 bg-primary text-primary-foreground font-medium transition-colors hover:bg-primary/90"
          >
            Get Started
          </Link>
          <a
            href="https://github.com/your-org/alchemy-rs"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center justify-center px-6 py-3 border border-border font-medium transition-colors hover:bg-accent"
          >
            View on GitHub
          </a>
        </div>
        <div className="font-mono text-sm text-muted-foreground bg-accent/50 inline-block px-4 py-2 border">
          cargo add alchemy
        </div>
      </div>
    </section>
  );
}
