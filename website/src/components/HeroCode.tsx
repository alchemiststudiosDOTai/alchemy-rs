import { useEffect, useState, useCallback } from "react";
import { cn } from "@/lib/utils";

const PROVIDERS = [
  { provider: "Anthropic", model: "claude-sonnet-4" },
  { provider: "OpenAI", model: "gpt-4o" },
  { provider: "Google", model: "gemini-2.0-flash" },
  { provider: "Bedrock", model: "anthropic.claude-3" },
  { provider: "Mistral", model: "mistral-large-latest" },
  { provider: "xAI", model: "grok-2" },
  { provider: "Groq", model: "llama-3.3-70b" },
];

const GLITCH_CHARS = "!@#$%^&*_+-=[]{}|;:<>?/~0123456789";
const TRANSITION_DURATION = 400;
const DISPLAY_DURATION = 3000;

function getRandomChar() {
  return GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)];
}

interface HeroCodeProps {
  className?: string;
}

export function HeroCode({ className }: HeroCodeProps) {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [displayProvider, setDisplayProvider] = useState(PROVIDERS[0].provider);
  const [displayModel, setDisplayModel] = useState(PROVIDERS[0].model);

  const glitchTransition = useCallback(
    (
      fromText: string,
      toText: string,
      onUpdate: (text: string) => void,
      onComplete: () => void
    ) => {
      const maxLen = Math.max(fromText.length, toText.length);
      const steps = 8;
      const stepDuration = TRANSITION_DURATION / steps;
      let step = 0;

      const interval = setInterval(() => {
        step++;

        if (step >= steps) {
          clearInterval(interval);
          onUpdate(toText);
          onComplete();
          return;
        }

        const progress = step / steps;
        let result = "";

        for (let i = 0; i < maxLen; i++) {
          const targetChar = toText[i] || "";
          const sourceChar = fromText[i] || "";

          if (progress > 0.7 && i < toText.length * progress) {
            result += targetChar;
          } else if (i < Math.max(fromText.length, toText.length * progress)) {
            result += getRandomChar();
          } else {
            result += sourceChar;
          }
        }

        onUpdate(result);
      }, stepDuration);

      return () => clearInterval(interval);
    },
    []
  );

  useEffect(() => {
    const interval = setInterval(() => {
      const nextIndex = (currentIndex + 1) % PROVIDERS.length;
      const nextProvider = PROVIDERS[nextIndex];
      const currentProvider = PROVIDERS[currentIndex];

      const cleanupProvider = glitchTransition(
        currentProvider.provider,
        nextProvider.provider,
        setDisplayProvider,
        () => {}
      );

      const cleanupModel = glitchTransition(
        currentProvider.model,
        nextProvider.model,
        setDisplayModel,
        () => {
          setCurrentIndex(nextIndex);
        }
      );

      return () => {
        cleanupProvider();
        cleanupModel();
      };
    }, DISPLAY_DURATION);

    return () => clearInterval(interval);
  }, [currentIndex, glitchTransition]);

  return (
    <pre className={cn("font-mono lg:text-xl leading-relaxed", className)}>
      <code>
        <Line>
          <Keyword>use</Keyword> alchemy::prelude::<Punct>*</Punct>;
        </Line>
        <Line />
        <Line>
          <Keyword>async</Keyword> <Keyword>fn</Keyword> <Fn>main</Fn>
          <Punct>()</Punct> <Punct>-&gt;</Punct> <Type>Result</Type>
          <Punct>&lt;()&gt;</Punct> <Punct>{"{"}</Punct>
        </Line>
        <Line>
          {"    "}<Keyword>let</Keyword> response <Punct>=</Punct>{" "}
          <Type>{displayProvider}</Type>
          <Punct>::</Punct>
          <Fn>new</Fn>
          <Punct>()</Punct>
        </Line>
        <Line>
          {"      "}<Punct>.</Punct>
          <Fn>model</Fn>
          <Punct>(</Punct>
          <String>"{displayModel}"</String>
          <Punct>)</Punct>
        </Line>
        <Line>
          {"      "}<Punct>.</Punct>
          <Fn>prompt</Fn>
          <Punct>(</Punct>
          <String>"Hello, world!"</String>
          <Punct>)</Punct>
        </Line>
        <Line>
          {"      "}<Punct>.</Punct>
          <Fn>send</Fn>
          <Punct>()</Punct>
        </Line>
        <Line>
          {"      "}<Punct>.</Punct>
          <Keyword>await</Keyword>
          <Punct>?;</Punct>
        </Line>
        <Line />
        <Line>
          {"    "}<Macro>println!</Macro>
          <Punct>(</Punct>
          <String>"{"{}"}"</String>, response.<Fn>text</Fn>
          <Punct>());</Punct>
        </Line>
        <Line>
          {"    "}<Keyword>Ok</Keyword>
          <Punct>(())</Punct>
        </Line>
        <Line>
          <Punct>{"}"}</Punct>
        </Line>
      </code>
    </pre>
  );
}

function Line({ children }: { children?: React.ReactNode }) {
  return <span className="block">{children || "\u00A0"}</span>;
}

function Keyword({ children }: { children: React.ReactNode }) {
  return <span className="text-[#d73a49] dark:text-[#c45a2c]">{children}</span>;
}

function Type({ children }: { children: React.ReactNode }) {
  return <span className="text-[#e36209] dark:text-[#d4976c]">{children}</span>;
}

function Fn({ children }: { children: React.ReactNode }) {
  return <span className="text-[#6f42c1] dark:text-[#e0e0e0]">{children}</span>;
}

function String({ children }: { children: React.ReactNode }) {
  return <span className="text-[#22863a] dark:text-[#c9a26d]">{children}</span>;
}

function Punct({ children }: { children: React.ReactNode }) {
  return <span className="text-[#24292e] dark:text-[#808080]">{children}</span>;
}

function Macro({ children }: { children: React.ReactNode }) {
  return <span className="text-[#6f42c1] dark:text-[#c45a2c]">{children}</span>;
}
