const providers = [
  { name: "Anthropic", logo: "anthropic" },
  { name: "OpenAI", logo: "openai" },
  { name: "Google", logo: "google" },
  { name: "AWS Bedrock", logo: "aws" },
  { name: "Mistral", logo: "mistral" },
  { name: "xAI", logo: "xai" },
  { name: "Groq", logo: "groq" },
  { name: "Cerebras", logo: "cerebras" },
  { name: "OpenRouter", logo: "openrouter" },
];

function ProviderItem({ name }: { name: string }) {
  return (
    <div className="flex items-center gap-3 px-8 text-muted-foreground whitespace-nowrap">
      <span className="text-lg font-medium">{name}</span>
    </div>
  );
}

export function ProvidersCarousel() {
  return (
    <section className="border-b py-6 overflow-hidden">
      <div className="flex items-center">
        <div className="shrink-0 px-12 text-sm border-r h-full py-2">
          Supported Providers
        </div>
        <div className="relative flex-1 overflow-hidden">
          <div className="flex animate-scroll">
            {[...providers, ...providers].map((provider, index) => (
              <ProviderItem
                key={`${provider.logo}-${index}`}
                name={provider.name}
              />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
