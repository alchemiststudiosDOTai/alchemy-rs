import { Link } from "react-router-dom";
import { ArrowRightIcon } from "lucide-react";
import { Header } from "@/components/Header";
import { HeroCode } from "@/components/HeroCode";
import { ProvidersCarousel } from "@/components/ProvidersCarousel";
import { FeaturesSection } from "@/components/FeaturesSection";
import { CodeExamplesSection } from "@/components/CodeExamplesSection";
import { ComparisonSection } from "@/components/ComparisonSection";
import { StatsSection } from "@/components/StatsSection";
import { CTASection } from "@/components/CTASection";
import { Footer } from "@/components/Footer";

export function Home() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header />

      <main className="container mx-auto border-x">
        <div className="container mx-auto border-b h-12 flex items-center justify-center gap-2 bg-primary text-background px-6">
          <span>version 0.1.0 is out!</span>
          <Link
            to="/docs"
            className="text-sm underline transition-all flex items-center gap-1"
          >
            <span>Install now</span>
            <ArrowRightIcon className="w-4 h-4" />
          </Link>
        </div>

        <section className="w-full border-b grid grid-cols-1 lg:grid-cols-2 lg:gap-12">
          <div className="border-r">
            <div className="w-full p-6 lg:p-12 border-b">
              <h1 className="text-4xl md:text-5xl lg:text-6xl font-semibold">
                Unified LLM API Abstraction Layer
                <br /> in Rust.
              </h1>
            </div>
            <div className="w-full p-6 lg:p-12">
              <p className="text-xl lg:text-2xl text-muted-foreground mb-8 lg:mb-12">
                Async streaming, type-safe tool calling, and prompt caching. One
                interface for Anthropic, OpenAI, Google, Bedrock, and more.
              </p>

              <div className="flex flex-col md:flex-row gap-4 lg:gap-6">
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
            </div>
          </div>

          <div className="border-t lg:border-t-0 lg:border-l p-6 lg:p-12 flex flex-col gap-6 overflow-hidden bg-secondary">
            <div className="w-full flex items-center font-mono text-accent-foreground opacity-50">
              src/alchemy.rs
            </div>
            <div className="w-full">
              <HeroCode />
            </div>
          </div>
        </section>

        <ProvidersCarousel />

        <FeaturesSection />

        <CodeExamplesSection />

        <ComparisonSection />

        <StatsSection />

        <CTASection />
      </main>

      <Footer />
    </div>
  );
}
