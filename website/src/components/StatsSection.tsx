import { cn } from "@/lib/utils";

const STATS = [
  { value: "9+", label: "Providers supported" },
  { value: "100%", label: "Async streaming" },
  { value: "0", label: "Runtime overhead" },
  { value: "MIT", label: "Open source license" },
];

export function StatsSection() {
  return (
    <section className="border-b">
      <div className="grid grid-cols-2 lg:grid-cols-4">
        {STATS.map((stat, index) => (
          <div
            key={stat.label}
            className={cn(
              "p-6 lg:p-12 text-center",
              index < 2 && "border-b lg:border-b-0",
              index % 2 === 0 && "border-r",
              index < 3 && "lg:border-r"
            )}
          >
            <div className="text-3xl lg:text-5xl font-bold lg:mb-2">
              {stat.value}
            </div>
            <div className="text-muted-foreground">{stat.label}</div>
          </div>
        ))}
      </div>
    </section>
  );
}
