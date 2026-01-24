import { useEffect, useRef, useState } from "react";
import {
  ZapIcon,
  ShieldCheckIcon,
  CodeIcon,
  RefreshCwIcon,
  Activity,
  ChevronRightSquare,
  GitCompare,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { FeaturesIllustration } from "./FeaturesIllustration";

const FEATURES = [
  {
    icon: Activity,
    title: "Streaming First",
    description:
      "All providers use async streams built on Tokio. Real-time responses with zero blocking.",
  },
  {
    icon: ShieldCheckIcon,
    title: "Type Safe",
    description:
      "Leverage Rust's type system with strong typing at every layer. Catch errors at compile time.",
  },
  {
    icon: ChevronRightSquare,
    title: "Tool Calling",
    description:
      "Type-safe function calling with automatic schema generation. Works consistently across all providers.",
  },
  {
    icon: GitCompare,
    title: "Prompt Caching",
    description:
      "Built-in support for prompt caching on supported providers. Reduce costs and latency automatically.",
  },
];

export function FeaturesSection() {
  const [activeFeature, setActiveFeature] = useState(0);
  const featureRefs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    const updateActiveFeature = () => {
      const viewportTop = window.scrollY;
      const offset = 100; // Small offset from top of viewport

      let activeIndex = 0;
      for (let i = 0; i < featureRefs.current.length; i++) {
        const ref = featureRefs.current[i];
        if (ref) {
          const rect = ref.getBoundingClientRect();
          const elementTop = rect.top + window.scrollY;

          // Find the first feature whose top is at or below the viewport top + offset
          // OR find the last feature that has scrolled past
          if (elementTop <= viewportTop + offset) {
            activeIndex = i;
          }
        }
      }
      setActiveFeature(activeIndex);
    };

    window.addEventListener("scroll", updateActiveFeature, { passive: true });
    updateActiveFeature(); // Initial check

    return () => window.removeEventListener("scroll", updateActiveFeature);
  }, []);

  return (
    <section className="border-b mb-12 lg:mb-24">
      {/* Section header */}
      <div className="border-b p-6 lg:p-12 relative flex flex-col lg:flex-row lg:justify-between">
        <h2 className="text-3xl lg:text-5xl mb-2 max-w-[80%] lg:max-w-1/3">
          Everything you need to build with LLMs
        </h2>
        <p className="text-muted-foreground text-xl lg:text-2xl max-w-full lg:max-w-1/3">
          A complete toolkit for integrating large language models into your
          Rust applications.
        </p>
      </div>

      {/* Two-column layout */}
      <div className="grid lg:grid-cols-2">
        {/* Left column: Features list */}
        <div className="border-r pt-6 lg:pt-12">
          {FEATURES.map((feature, index) => (
            <div
              key={feature.title}
              ref={(el) => {
                featureRefs.current[index] = el;
              }}
              className={cn(
                "border-b transition-colors duration-300 mb-6 lg:mb-12 border-t last:border-mb-0",
                activeFeature === index && "bg-muted/50"
              )}
            >
              <div className="ml-6 lg:ml-12 h-12 w-16 border-x"></div>
              <div className="w-full border-y pl-6 lg:pl-12">
                <div className="h-16 w-16 border-x flex items-center justify-center">
                <feature.icon className={cn("w-8 h-8", activeFeature === index ? "text-primary" : "text-muted-foreground")} />
                </div>
              </div>
              <div className="w-full pl-6 lg:pl-12 py-8 lg:py-12">
              <h3 className="text-2xl lg:text-3xl mb-2">{feature.title}</h3>
              <p className="text-muted-foreground text-lg lg:text-xl max-w-[90%]">
                {feature.description}
              </p>
              </div>
            </div>
          ))}
        </div>

        {/* Right column: Sticky illustration */}
        <div className="hidden lg:block relative">
          {/* Dots background - covers entire right column */}
          <div
            className="absolute inset-0 opacity-20"
            style={{
              backgroundImage: `radial-gradient(circle, currentColor 1px, transparent 1px)`,
              backgroundSize: "24px 24px",
            }}
          />
          <div className="sticky top-0 flex items-center justify-center p-12">
            <FeaturesIllustration
              activeFeature={activeFeature}
              className="max-w-full max-h-[80vh] relative z-10"
            />
          </div>
        </div>
      </div>
    </section>
  );
}
