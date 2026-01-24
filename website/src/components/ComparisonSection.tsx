import { CheckIcon, XIcon } from "lucide-react";

const COMPARISON_FEATURES = [
  { name: "Streaming responses", alchemy: true, raw: true },
  { name: "Unified API across providers", alchemy: true, raw: false },
  { name: "Type-safe tool calling", alchemy: true, raw: false },
  { name: "Automatic retry & error handling", alchemy: true, raw: false },
  { name: "Prompt caching", alchemy: true, raw: "partial" as const },
  { name: "Provider switching", alchemy: true, raw: false },
  { name: "Message history management", alchemy: true, raw: false },
  { name: "Usage tracking", alchemy: true, raw: "partial" as const },
];

export function ComparisonSection() {
  return (
    <section className="border-y">
      {/* Two-column layout */}
      <div className="grid lg:grid-cols-2">
        {/* Left column: Title and description */}
        <div className="border-b lg:border-b-0 lg:border-r">
          <div className="p-6 lg:p-12">
            <h2 className="text-3xl lg:text-5xl mb-2 lg:mb-4">Why Alchemy?</h2>
            <p className="text-muted-foreground text-lg lg:text-xl max-w-[90%]">
              Stop wrestling with provider-specific SDKs. Alchemy handles the
              complexity.
            </p>
          </div>
        </div>

        {/* Right column: Comparison table */}
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b">
                <th className="text-left p-4 lg:p-6 font-medium w-1/2">Feature</th>
                <th className="text-center p-4 lg:p-6 font-medium border-x bg-accent/30 w-1/4">
                  Alchemy
                </th>
                <th className="text-center p-4 lg:p-6 font-medium">
                  Raw SDKs
                </th>
              </tr>
            </thead>
            <tbody>
              {COMPARISON_FEATURES.map((feature, index) => (
                <tr
                  key={feature.name}
                  className={
                    index < COMPARISON_FEATURES.length - 1 ? "border-b" : ""
                  }
                >
                  <td className="p-4 lg:p-6">{feature.name}</td>
                  <td className="p-4 lg:p-6 text-center border-x bg-accent/30">
                    {feature.alchemy ? (
                      <CheckIcon className="w-5 h-5 text-primary mx-auto" />
                    ) : (
                      <XIcon className="w-5 h-5 text-muted-foreground mx-auto" />
                    )}
                  </td>
                  <td className="p-4 lg:p-6 text-center">
                    {feature.raw === true ? (
                      <CheckIcon className="w-5 h-5 text-primary/50 mx-auto" />
                    ) : feature.raw === "partial" ? (
                      <span className="text-muted-foreground/50 text-sm">Varies</span>
                    ) : (
                      <XIcon className="w-5 h-5 text-muted-foreground/50 mx-auto" />
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
