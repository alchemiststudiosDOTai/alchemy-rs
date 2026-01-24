import { Link } from "react-router-dom";
import { Header } from "../components/Header";
import { ArrowRightIcon } from "lucide-react";

export function Home() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header />

      <main className="container mx-auto border-x">
        <div className="container mx-auto border-b h-12 flex items-center justify-center gap-2 bg-primary text-background px-6">
          <span>version 0.1.0 is out!</span>
          <Link to="/docs" className="text-sm underline transition-all flex items-center gap-1">
            <span>Check it out</span>
            <ArrowRightIcon className="w-4 h-4" />
          </Link>
        </div>

        <section className="w-full border-b px-12 grid grid-cols-1 md:grid-cols-2 gap-12">
          <div className="py-12">

          <h1 className="text-5xl md:text-6xl font-bold mb-6">
            Unified LLM API Abstraction Layer in Rust.
          </h1>
          <p className="text-xl text-muted-foreground mb-8">
            Unified LLM API abstraction layer in Rust. Support for 8+ providers
            through 4 main API interfaces.
          </p>

          <div className="flex gap-4">
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

          <div className="border-t md:border-t-0 md:border-l py-12"></div>
        </section>

        <div className="mt-24 grid md:grid-cols-3 gap-8">
          <div className="p-6 border border-border">
            <h3 className="font-semibold mb-2">Multi-Provider</h3>
            <p className="text-sm text-muted-foreground">
              Anthropic, OpenAI, Google, AWS Bedrock, Mistral, xAI, Groq,
              Cerebras, and OpenRouter support.
            </p>
          </div>
          <div className="p-6 border border-border">
            <h3 className="font-semibold mb-2">Streaming First</h3>
            <p className="text-sm text-muted-foreground">
              All providers use async streams. Built on Tokio for maximum
              performance.
            </p>
          </div>
          <div className="p-6 border border-border">
            <h3 className="font-semibold mb-2">Type Safe</h3>
            <p className="text-sm text-muted-foreground">
              Leverages Rust's type system. Strong typing at every layer with
              comprehensive error handling.
            </p>
          </div>
        </div>
      </main>
    </div>
  );
}
