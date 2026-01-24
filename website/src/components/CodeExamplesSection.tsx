import { useState, useEffect } from "react";
import { ChevronRightIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { CodeBlock } from "./primitives/animate/code-block";

const CODE_EXAMPLES = [
  {
    key: "basic",
    title: "Basic Usage",
    description: "Simple prompt completion with any provider",
    code: `use alchemy::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let response = Anthropic::new()
        .model("claude-sonnet-4-20250514")
        .prompt("Explain quantum computing")
        .send()
        .await?;

    println!("{}", response.text());
    Ok(())
}`,
  },
  {
    key: "streaming",
    title: "Streaming",
    description: "Real-time responses with async streams",
    code: `use alchemy::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut stream = OpenAI::new()
        .model("gpt-4o")
        .prompt("Write a poem about Rust")
        .stream()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            Event::Text(text) => print!("{text}"),
            Event::Done(msg) => println!("\\n\\nTokens: {}", msg.usage.total),
            _ => {}
        }
    }
    Ok(())
}`,
  },
  {
    key: "tools",
    title: "Tool Calling",
    description: "Function calling with automatic schema generation",
    code: `use alchemy::prelude::*;

#[derive(Tool)]
#[tool(description = "Get current weather for a location")]
struct GetWeather {
    #[tool(description = "City name")]
    location: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let response = Google::new()
        .model("gemini-2.0-flash")
        .tools([GetWeather::definition()])
        .prompt("What's the weather in Tokyo?")
        .send()
        .await?;

    for call in response.tool_calls() {
        println!("Called: {} with {:?}", call.name, call.args);
    }
    Ok(())
}`,
  },
  {
    key: "multimodal",
    title: "Vision",
    description: "Multimodal inputs with images and text",
    code: `use alchemy::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let image = Content::image_url(
        "https://example.com/chart.png"
    );

    let response = Anthropic::new()
        .model("claude-sonnet-4-20250514")
        .messages([
            Message::user([
                image,
                Content::text("Describe this chart"),
            ])
        ])
        .send()
        .await?;

    println!("{}", response.text());
    Ok(())
}`,
  },
];

export function CodeExamplesSection() {
  const [activeIndex, setActiveIndex] = useState(0);
  const [theme, setTheme] = useState<"light" | "dark">("light");

  useEffect(() => {
    const root = document.documentElement;
    const updateTheme = () => {
      setTheme(root.classList.contains("dark") ? "dark" : "light");
    };
    updateTheme();

    const observer = new MutationObserver(updateTheme);
    observer.observe(root, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  return (
    <section className="border-y mb-12">
      {/* Two-column layout */}
      <div className="grid lg:grid-cols-2">
        {/* Left column: Title, description, and options */}
        <div className="border-b lg:border-b-0 lg:border-r">
          {/* Section header */}
          <div className="p-6 lg:p-12 border-b">
            <h2 className="text-3xl lg:text-5xl mb-2 lg:mb-4">Simple, expressive API</h2>
            <p className="text-muted-foreground text-lg lg:text-xl max-w-[90%]">
              From basic prompts to complex tool-calling workflows, Alchemy makes it
              easy.
            </p>
          </div>

          {/* Example options */}
          {CODE_EXAMPLES.map((example, index) => (
            <button
              key={example.key}
              onClick={() => setActiveIndex(index)}
              className={cn(
                "w-full text-left border-b last:border-b-0 transition-colors cursor-pointer",
                activeIndex === index ? "bg-muted/50" : "hover:bg-muted/25"
              )}
            >
              <div className="p-6 lg:px-12 flex items-center justify-between gap-4">
                <div>
                  <h3
                    className={cn(
                      "text-xl mb-1 transition-colors",
                      activeIndex === index
                        ? "text-foreground"
                        : "text-muted-foreground"
                    )}
                  >
                    {example.title}
                  </h3>
                  <p className="text-muted-foreground">{example.description}</p>
                </div>
                <ChevronRightIcon
                  className={cn(
                    "w-5 h-5 flex-shrink-0 transition-colors",
                    activeIndex === index
                      ? "text-foreground"
                      : "text-muted-foreground"
                  )}
                />
              </div>
            </button>
          ))}
        </div>

        {/* Right column: Code display */}
        <div className="p-8 lg:p-12 overflow-x-auto bg-secondary">
          <CodeBlock
            code={CODE_EXAMPLES[activeIndex].code}
            lang="rust"
            theme={theme}
            className="font-mono text-sm leading-relaxed [&_pre]:!bg-transparent [&_code]:!bg-transparent"
          />
        </div>
      </div>
    </section>
  );
}
